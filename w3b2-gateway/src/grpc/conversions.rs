use crate::grpc::proto::w3b2::bridge::gateway;
use w3b2_connector::events as ConnectorEvents;

impl From<ConnectorEvents::BridgeEvent> for gateway::BridgeEvent {
    fn from(event: ConnectorEvents::BridgeEvent) -> Self {
        let event_oneof = match event {
            ConnectorEvents::BridgeEvent::AdminProfileRegistered(e) => {
                Some(gateway::bridge_event::Event::AdminProfileRegistered(
                    gateway::AdminProfileRegistered {
                        authority: e.authority.to_string(),
                        communication_pubkey: e.communication_pubkey.to_string(),
                        ts: e.ts,
                    },
                ))
            }
            ConnectorEvents::BridgeEvent::AdminCommKeyUpdated(e) => Some(
                gateway::bridge_event::Event::AdminCommKeyUpdated(gateway::AdminCommKeyUpdated {
                    authority: e.authority.to_string(),
                    new_comm_pubkey: e.new_comm_pubkey.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::AdminPricesUpdated(e) => Some(
                gateway::bridge_event::Event::AdminPricesUpdated(gateway::AdminPricesUpdated {
                    authority: e.authority.to_string(),
                    new_prices: e
                        .new_prices
                        .into_iter()
                        .map(|p| gateway::PriceEntry {
                            command_id: p.command_id as u32,
                            price: p.price,
                        })
                        .collect(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::AdminFundsWithdrawn(e) => Some(
                gateway::bridge_event::Event::AdminFundsWithdrawn(gateway::AdminFundsWithdrawn {
                    authority: e.authority.to_string(),
                    amount: e.amount,
                    destination: e.destination.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::AdminProfileClosed(e) => Some(
                gateway::bridge_event::Event::AdminProfileClosed(gateway::AdminProfileClosed {
                    authority: e.authority.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::AdminCommandDispatched(e) => {
                Some(gateway::bridge_event::Event::AdminCommandDispatched(
                    gateway::AdminCommandDispatched {
                        sender: e.sender.to_string(),
                        target_user_authority: e.target_user_authority.to_string(),
                        command_id: e.command_id as u32,
                        payload: e.payload,
                        ts: e.ts,
                    },
                ))
            }
            ConnectorEvents::BridgeEvent::UserProfileCreated(e) => Some(
                gateway::bridge_event::Event::UserProfileCreated(gateway::UserProfileCreated {
                    authority: e.authority.to_string(),
                    target_admin: e.target_admin.to_string(),
                    communication_pubkey: e.communication_pubkey.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::UserCommKeyUpdated(e) => Some(
                gateway::bridge_event::Event::UserCommKeyUpdated(gateway::UserCommKeyUpdated {
                    authority: e.authority.to_string(),
                    new_comm_pubkey: e.new_comm_pubkey.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::UserFundsDeposited(e) => Some(
                gateway::bridge_event::Event::UserFundsDeposited(gateway::UserFundsDeposited {
                    authority: e.authority.to_string(),
                    amount: e.amount,
                    new_deposit_balance: e.new_deposit_balance,
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::UserFundsWithdrawn(e) => Some(
                gateway::bridge_event::Event::UserFundsWithdrawn(gateway::UserFundsWithdrawn {
                    authority: e.authority.to_string(),
                    amount: e.amount,
                    destination: e.destination.to_string(),
                    new_deposit_balance: e.new_deposit_balance,
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::UserProfileClosed(e) => Some(
                gateway::bridge_event::Event::UserProfileClosed(gateway::UserProfileClosed {
                    authority: e.authority.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::UserCommandDispatched(e) => {
                Some(gateway::bridge_event::Event::UserCommandDispatched(
                    gateway::UserCommandDispatched {
                        sender: e.sender.to_string(),
                        target_admin_authority: e.target_admin_authority.to_string(),
                        command_id: e.command_id as u32,
                        price_paid: e.price_paid,
                        payload: e.payload,
                        ts: e.ts,
                    },
                ))
            }
            ConnectorEvents::BridgeEvent::OffChainActionLogged(e) => Some(
                gateway::bridge_event::Event::OffChainActionLogged(gateway::OffChainActionLogged {
                    actor: e.actor.to_string(),
                    session_id: e.session_id,
                    action_code: e.action_code as u32,
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEvent::Unknown => None,
        };

        Self { event: event_oneof }
    }
}
