// Copyright (C) 2024 [Kulpreet Singh]
//
//  This file is part of P2Poolv2
//
// P2Poolv2 is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// P2Poolv2 is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// P2Poolv2. If not, see <https://www.gnu.org/licenses/>.

use crate::node::behaviour::request_response::RequestResponseEvent;
use crate::node::messages::Message;
use crate::node::SwarmSend;
#[mockall_double::double]
use crate::shares::chain::actor::ChainHandle;
use std::error::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Handle request-response events, these are events that are generated by the request-response protocol
/// The caller spawns a new task to handle the event to allow concurrent handling of events
pub async fn handle_request_response_event(
    event: RequestResponseEvent<Message, Message>,
    chain_handle: ChainHandle,
    swarm_tx: mpsc::Sender<SwarmSend>,
) -> Result<(), Box<dyn Error>> {
    info!("Request-response event: {:?}", event);
    match event {
        RequestResponseEvent::Message {
            peer,
            message:
                libp2p::request_response::Message::Request {
                    request_id: _,
                    request,
                    channel: _,
                },
        } => {
            debug!("Received request from peer: {}", peer);
            if let Err(e) = handle_request(peer, request, chain_handle, swarm_tx).await {
                error!("Failed to handle request: {}", e);
                return Err("Error handling request".into());
            }
        }
        RequestResponseEvent::Message {
            peer,
            message:
                libp2p::request_response::Message::Response {
                    request_id,
                    response: _,
                },
        } => {
            debug!(
                "Received response for request: {} from peer: {}",
                request_id, peer
            );
        }
        RequestResponseEvent::OutboundFailure {
            peer,
            request_id,
            error,
        } => {
            debug!(
                "Outbound failure from peer: {peer}, request_id: {request_id}, error: {error:?}"
            );
        }
        RequestResponseEvent::InboundFailure {
            peer,
            request_id,
            error,
        } => {
            debug!("Inbound failure from peer: {peer}, request_id: {request_id}, error: {error:?}");
        }
        RequestResponseEvent::ResponseSent { peer, request_id } => {
            debug!("Response sent to peer: {peer}, request_id: {request_id}");
        }
    }
    Ok(())
}

async fn handle_request(
    peer: libp2p::PeerId,
    request: Message,
    chain_handle: ChainHandle,
    swarm_tx: mpsc::Sender<SwarmSend>,
) -> Result<(), Box<dyn Error>> {
    info!("Handling request from peer: {}", peer);
    match request {
        Message::ShareBlock(share_block) => {
            info!("Received share block: {:?}", share_block);
            if let Err(e) = share_block.miner_share.validate() {
                error!("Share block validation failed: {}", e);
                return Err("Share block validation failed".into());
            }
            if let Err(e) = chain_handle.add_share(share_block.clone()).await {
                error!("Failed to add share: {}", e);
                return Err("Error adding share to chain".into());
            }
            let buf = Message::ShareBlock(share_block).cbor_serialize().unwrap();
            if let Err(e) = swarm_tx.send(SwarmSend::Gossip(buf)).await {
                error!("Failed to send share: {}", e);
                return Err("Error sending share to network".into());
            }
            Ok(())
        }
        Message::Workbase(workbase) => {
            info!("Received workbase: {:?}", workbase);
            if let Err(e) = chain_handle.add_workbase(workbase.clone()).await {
                error!("Failed to store workbase: {}", e);
                return Err("Error storing workbase".into());
            }
            let buf = Message::Workbase(workbase).cbor_serialize().unwrap();
            if let Err(e) = swarm_tx.send(SwarmSend::Gossip(buf)).await {
                error!("Failed to send share: {}", e);
                return Err("Error sending share to network".into());
            }
            Ok(())
        }
        _ => {
            info!("Received unknown request: {:?}", request);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[mockall_double::double]
    use crate::shares::chain::actor::ChainHandle;
    use crate::shares::miner_message::Gbt;
    use crate::shares::miner_message::MinerWorkbase;
    use crate::shares::ShareBlock;
    use crate::test_utils::simple_miner_share;
    use crate::test_utils::simple_miner_workbase;
    use mockall::predicate::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_handle_share_block_request() {
        let mut chain_handle = ChainHandle::default();
        let (swarm_tx, mut swarm_rx) = mpsc::channel(32);
        let peer_id = libp2p::PeerId::random();

        // Create test share block
        let share_block = ShareBlock {
            blockhash: "0000000086704a35f17580d06f76d4c02d2b1f68774800675fb45f0411205bb5"
                .parse()
                .unwrap(),
            prev_share_blockhash: None,
            uncles: vec![],
            miner_pubkey: "020202020202020202020202020202020202020202020202020202020202020202"
                .parse()
                .unwrap(),
            tx_hashes: vec![],
            miner_share: simple_miner_share(
                Some(7452731920372203525),
                Some(1),
                Some(dec!(1.0)),
                Some(dec!(1.9041854952356509)),
            ),
        };

        // Set up mock expectations
        chain_handle
            .expect_add_share()
            .with(eq(share_block.clone()))
            .returning(|_| Ok(()));

        // Test handle_request directly without request_id
        let result = handle_request(
            peer_id,
            Message::ShareBlock(share_block.clone()),
            chain_handle,
            swarm_tx,
        )
        .await;

        assert!(result.is_ok());

        // Verify gossip message was sent
        if let Some(SwarmSend::Gossip(buf)) = swarm_rx.try_recv().ok() {
            let message = Message::cbor_deserialize(&buf).unwrap();
            match message {
                Message::ShareBlock(received_share) => {
                    assert_eq!(received_share.blockhash, share_block.blockhash);
                    assert_eq!(
                        received_share.miner_share.diff,
                        share_block.miner_share.diff
                    );
                }
                _ => panic!("Expected ShareBlock message"),
            }
        } else {
            panic!("Expected gossip message");
        }
    }

    #[tokio::test]
    async fn test_handle_request_share_block_error() {
        let peer_id = libp2p::PeerId::random();
        let (swarm_tx, mut swarm_rx) = mpsc::channel(32);
        let mut chain_handle = ChainHandle::default();

        let share_block = ShareBlock {
            blockhash: "0000000086704a35f17580d06f76d4c02d2b1f68774800675fb45f0411205bb5"
                .parse()
                .unwrap(),
            prev_share_blockhash: None,
            uncles: vec![],
            miner_pubkey: "020202020202020202020202020202020202020202020202020202020202020202"
                .parse()
                .unwrap(),
            tx_hashes: vec![],
            miner_share: simple_miner_share(
                Some(7452731920372203525),
                Some(1),
                Some(dec!(1.0)),
                Some(dec!(1.9041854952356509)),
            ),
        };

        // Set up mock to return error
        chain_handle
            .expect_add_share()
            .with(eq(share_block.clone()))
            .returning(|_| Err("Failed to add share".into()));

        let result = handle_request(
            peer_id,
            Message::ShareBlock(share_block),
            chain_handle,
            swarm_tx,
        )
        .await;

        assert!(result.is_err());

        // Verify no gossip message was sent
        assert!(swarm_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_handle_request_workbase_success() {
        let peer_id = libp2p::PeerId::random();
        let (swarm_tx, mut swarm_rx) = mpsc::channel(32);
        let mut chain_handle = ChainHandle::default();

        let workbase = simple_miner_workbase();

        // Set up mock to return success
        chain_handle
            .expect_add_workbase()
            .with(eq(workbase.clone()))
            .returning(|_| Ok(()));

        let result =
            handle_request(peer_id, Message::Workbase(workbase), chain_handle, swarm_tx).await;

        assert!(result.is_ok());

        // Verify gossip message was sent
        if let Some(SwarmSend::Gossip(_)) = swarm_rx.try_recv().ok() {
            // Success
        } else {
            panic!("Expected gossip message");
        }
    }

    #[tokio::test]
    async fn test_handle_request_workbase_error() {
        let peer_id = libp2p::PeerId::random();
        let (swarm_tx, mut swarm_rx) = mpsc::channel(32);
        let mut chain_handle = ChainHandle::default();

        let workbase = simple_miner_workbase();

        // Set up mock to return error
        chain_handle
            .expect_add_workbase()
            .with(eq(workbase.clone()))
            .returning(|_| Err("Failed to add workbase".into()));

        let result =
            handle_request(peer_id, Message::Workbase(workbase), chain_handle, swarm_tx).await;

        assert!(result.is_err());

        // Verify no gossip message was sent
        assert!(swarm_rx.try_recv().is_err());
    }
}
