use super::*;

#[derive(Debug)]
#[event]
pub struct AdminRegistered {
    pub admin: Pubkey,
    pub initial_funding: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct UserRegistered {
    pub user: Pubkey,
    pub initial_balance: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct AdminDeactivated {
    pub admin: Pubkey,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct UserDeactivated {
    pub user: Pubkey,
    pub ts: i64,
}

#[event]
pub struct CommKeyUpdated {
    pub pda_owner: Pubkey,
    pub new_comm_pubkey: Pubkey,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct FundingRequested {
    pub user_wallet: Pubkey,
    pub user_comm_pubkey: Pubkey,
    pub target_admin: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct FundingApproved {
    pub user_wallet: Pubkey,
    pub approved_by: Pubkey,
    pub admin_comm_pubkey: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct CommandEvent {
    pub sender: Pubkey,
    pub target: Pubkey,
    pub command_id: u64,
    pub mode: CommandMode,
    pub payload: Vec<u8>,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct HttpActionEvent {
    pub actor: Pubkey,
    pub session_id: u64,
    pub action_code: u16,
    pub ts: i64,
}
