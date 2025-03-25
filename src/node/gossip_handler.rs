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

use crate::node::p2p_message_handlers::receivers::share_blocks::handle_share_block;
use crate::node::Message;
#[mockall_double::double]
use crate::shares::chain::actor::ChainHandle;
use crate::utils::time_provider::SystemTimeProvider;
use libp2p::{gossipsub, PeerId};
use std::error::Error;
use tracing::{debug, error, info};

/// Handle gossipsub events, these are events that are generated by the gossipsub protocol
/// We need to handle all events that can be gossiped. Currently, we gossip:
/// 1. Workbase(MinerWorkbase)
/// 2. UserWorkbase(UserWorkbase)
/// 3. MiningShare(ShareBlock)
pub async fn handle_gossipsub_event(
    event: gossipsub::Event,
    chain_handle: ChainHandle,
) -> Result<(), Box<dyn Error>> {
    debug!("Gossipsub event: {:?}", event);
    match event {
        gossipsub::Event::Message {
            propagation_source,
            message_id: _,
            message,
        } => {
            let message = Message::cbor_deserialize(&message.data).unwrap();
            if let Err(e) = handle_gossip_message(message, chain_handle, propagation_source).await {
                error!("Failed to handle gossip message: {}", e);
                return Err("Failed to handle gossip message".into());
            }
            Ok(())
        }
        _ => {
            // Do nothing for all other gossip events
            Ok(())
        }
    }
}

async fn handle_gossip_message(
    message: Message,
    chain_handle: ChainHandle,
    peer_id: PeerId,
) -> Result<(), Box<dyn Error>> {
    info!(
        "Handling gossip message: {:?} from peer: {}",
        message, peer_id
    );
    match message {
        Message::Workbase(workbase) => {
            info!("Handling workbase: {:?}", workbase);
            if let Err(e) = chain_handle.add_workbase(workbase).await {
                error!("Failed to add workbase: {}", e);
                return Err("Failed to add workbase".into());
            }
            Ok(())
        }
        Message::UserWorkbase(user_workbase) => {
            info!("Handling user workbase: {:?}", user_workbase);
            if let Err(e) = chain_handle.add_user_workbase(user_workbase).await {
                error!("Failed to store user workbase: {}", e);
                return Err("Failed to store user workbase".into());
            }
            Ok(())
        }
        Message::MiningShare(mining_share) => {
            info!("Handling mining share: {:?}", mining_share);
            let time_provider = SystemTimeProvider {};
            if let Err(e) = handle_share_block(mining_share, chain_handle, &time_provider).await {
                error!("Failed to add share: {}", e);
                return Err(format!("Failed to add share, Error: {}", e).into());
            }
            Ok(())
        }
        _ => {
            // Quietly skip all other Message types
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shares::miner_message::{CkPoolMessage, MinerWorkbase, UserWorkbase};
    use crate::test_utils::TestBlockBuilder;
    use libp2p::gossipsub::{MessageId, TopicHash};
    use libp2p::PeerId;

    #[tokio::test]
    async fn test_handle_gossip_event() {
        let mut mock_chain = ChainHandle::default();

        let json_str = include_str!("../../tests/test_data/simple_miner_workbase.json");
        let workbase: MinerWorkbase = serde_json::from_str(&json_str).unwrap();

        mock_chain
            .expect_add_workbase()
            .with(mockall::predicate::eq(workbase.clone()))
            .times(1)
            .returning(|_| Ok(()));

        let message = Message::Workbase(workbase).cbor_serialize().unwrap();

        let event = gossipsub::Event::Message {
            propagation_source: PeerId::random(),
            message_id: MessageId::new(b"test"),
            message: gossipsub::Message {
                source: Some(PeerId::random()),
                data: message,
                sequence_number: Some(0),
                topic: TopicHash::from_raw("share"),
            },
        };

        let result = handle_gossipsub_event(event, mock_chain).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_gossip_event_should_handle_error_from_chain_handle() {
        let mut mock_chain = ChainHandle::default();

        let json_str = include_str!("../../tests/test_data/simple_miner_workbase.json");
        let workbase: MinerWorkbase = serde_json::from_str(&json_str).unwrap();

        mock_chain
            .expect_add_workbase()
            .with(mockall::predicate::eq(workbase.clone()))
            .times(1)
            .returning(|_| Err("Failed to add workbase".into()));

        let message = Message::Workbase(workbase).cbor_serialize().unwrap();

        let event = gossipsub::Event::Message {
            propagation_source: PeerId::random(),
            message_id: MessageId::new(b"test"),
            message: gossipsub::Message {
                source: Some(PeerId::random()),
                data: message,
                sequence_number: Some(0),
                topic: TopicHash::from_raw("share"),
            },
        };

        let result = handle_gossipsub_event(event, mock_chain).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to handle gossip message"
        );
    }

    #[tokio::test]
    async fn test_handle_gossip_message_workbase() {
        let mut mock_chain = ChainHandle::default();

        let json_str = include_str!("../../tests/test_data/simple_miner_workbase.json");
        let workbase: MinerWorkbase = serde_json::from_str(&json_str).unwrap();

        mock_chain
            .expect_add_workbase()
            .with(mockall::predicate::eq(workbase.clone()))
            .times(1)
            .returning(|_| Ok(()));

        let result =
            handle_gossip_message(Message::Workbase(workbase), mock_chain, PeerId::random()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_gossip_message_workbase_handle_error_from_chain_handle() {
        let mut mock_chain = ChainHandle::default();

        let json_str = include_str!("../../tests/test_data/simple_miner_workbase.json");
        let workbase: MinerWorkbase = serde_json::from_str(&json_str).unwrap();

        mock_chain
            .expect_add_workbase()
            .with(mockall::predicate::eq(workbase.clone()))
            .times(1)
            .returning(|_| Err("Failed to add workbase".into()));

        let result =
            handle_gossip_message(Message::Workbase(workbase), mock_chain, PeerId::random()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Failed to add workbase");
    }

    #[tokio::test]
    async fn test_handle_gossip_message_user_workbase() {
        let mut mock_chain = ChainHandle::default();

        let json_str = include_str!("../../tests/test_data/validation/userworkbases.json");
        let userworkbases: Vec<CkPoolMessage> = serde_json::from_str(&json_str).unwrap();
        let user_workbases = userworkbases
            .into_iter()
            .filter_map(|msg| match msg {
                CkPoolMessage::UserWorkbase(w) => Some(w),
                _ => None,
            })
            .collect::<Vec<UserWorkbase>>();
        let user_workbase = user_workbases[0].clone();

        mock_chain
            .expect_add_user_workbase()
            .with(mockall::predicate::eq(user_workbase.clone()))
            .times(1)
            .returning(|_| Ok(()));

        let result = handle_gossip_message(
            Message::UserWorkbase(user_workbase),
            mock_chain,
            PeerId::random(),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_gossip_message_user_workbase_handle_error_from_chain_handle() {
        let mut mock_chain = ChainHandle::default();

        let json_str = include_str!("../../tests/test_data/validation/userworkbases.json");
        let userworkbases: Vec<CkPoolMessage> = serde_json::from_str(&json_str).unwrap();
        let user_workbases = userworkbases
            .into_iter()
            .filter_map(|msg| match msg {
                CkPoolMessage::UserWorkbase(w) => Some(w),
                _ => None,
            })
            .collect::<Vec<UserWorkbase>>();
        let user_workbase = user_workbases[0].clone();

        mock_chain
            .expect_add_user_workbase()
            .with(mockall::predicate::eq(user_workbase.clone()))
            .times(1)
            .returning(|_| Err("Failed to store user workbase".into()));

        let result = handle_gossip_message(
            Message::UserWorkbase(user_workbase),
            mock_chain,
            PeerId::random(),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to store user workbase"
        );
    }

    #[tokio::test]
    async fn test_handle_gossip_message_mining_share_calls_handle_share_block_but_returns_error_with_validation_error(
    ) {
        let mock_chain = ChainHandle::default();

        let share_block = TestBlockBuilder::new()
            .blockhash("00".repeat(32).as_str())
            .prev_share_blockhash("00".repeat(32).as_str().into())
            .build();

        let result = handle_gossip_message(
            Message::MiningShare(share_block),
            mock_chain,
            PeerId::random(),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to add share, Error: Share block validation failed"
        );
    }
}
