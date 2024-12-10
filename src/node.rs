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

use futures::StreamExt;
use libp2p::{
    swarm::SwarmEvent,
    Multiaddr,
    Swarm,
    kad::{Event as KademliaEvent, QueryResult},
};
use tracing::{debug, info, error};
use std::time::Duration;
use crate::config::Config;
use crate::behaviour::{P2PoolBehaviour, P2PoolBehaviourEvent};
use libp2p::identify;

pub struct Node {
    swarm: Swarm<P2PoolBehaviour>,
}

impl Node {
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let id_keys = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = id_keys.public().to_peer_id();

        let behavior = match P2PoolBehaviour::new(&id_keys) {
            Ok(behavior) => behavior,
            Err(err) => {
                error!("Failed to create P2PoolBehaviour: {}", err);
                std::process::exit(1);
            }
        };

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(id_keys)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| behavior)?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
            .build();

        swarm.listen_on(config.network.listen_address.parse()?)?;

        for peer_addr in &config.network.dial_peers {
            match peer_addr.parse::<Multiaddr>() {
                Ok(remote) => {
                    if let Err(e) = swarm.dial(remote) {
                        debug!("Failed to dial {}: {}", peer_addr, e);
                    } else {
                        info!("Dialed {}", peer_addr);
                    }
                }
                Err(e) => debug!("Invalid multiaddr {}: {}", peer_addr, e),
            }
        }

        Ok(Self { swarm })
    }

    pub async fn run(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    info!("Connected to peer: {peer_id}");
                },
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    info!("Disconnected from peer: {peer_id}");
                    self.swarm.behaviour_mut().remove_peer(&peer_id);
                },
                SwarmEvent::Behaviour(event) => {
                    match event {
                        P2PoolBehaviourEvent::Identify(identify::Event::Received { peer_id, info }) => {
                            info!("Identified Peer {} with protocol version {}", peer_id, info.protocol_version);
                            // Add the peer's advertised addresses to Kademlia
                            for addr in info.listen_addrs {
                                self.swarm.behaviour_mut().add_address(peer_id, addr.clone());
                            }
                        },
                        // P2PoolBehaviourEvent::Gossipsub(gossip_event) => {
                        //     debug!("Gossipsub event: {:?}", gossip_event);
                        // },
                        P2PoolBehaviourEvent::Kademlia(kad_event) => {
                            match kad_event {
                                KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => {
                                    info!("Routing updated for peer: {peer}, is_new_peer: {is_new_peer}, addresses: {addresses:?}, bucket_range: {bucket_range:?}, old_peer: {old_peer:?}");
                                },
                                KademliaEvent::OutboundQueryProgressed { result, .. } => {
                                    match result {
                                        QueryResult::GetClosestPeers(Ok(ok)) => {
                                            info!("Got closest peers: {:?}", ok.peers);
                                        },
                                        QueryResult::GetClosestPeers(Err(err)) => {
                                            debug!("Failed to get closest peers: {err}");
                                        },
                                        _ => debug!("Other query result: {:?}", result),
                                    }
                                },
                                _ => debug!("Other Kademlia event: {:?}", kad_event),
                            }
                        },
                        // P2PoolBehaviourEvent::Ping(ping_event) => {
                        //     debug!("Ping event: {:?}", ping_event);
                        // },
                        P2PoolBehaviourEvent::Identify(identify_event) => {
                            debug!("Other Identify event: {:?}", identify_event);
                        },
                    }
                },
                _ => {}
            }
        }
    }
} 