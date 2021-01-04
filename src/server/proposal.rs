use std::sync::mpsc::{self, Receiver, SyncSender};

use raft::{prelude::*};

#[derive(Debug)]
pub struct Proposal {
    pub normal: Option<(String, String)>,
    // key is an u16 integer, and value is a string.
    pub conf_change: Option<ConfChange>,
    // conf change.
    pub transfer_leader: Option<u64>,
    // If it's proposed, it will be set to the index of the entry.
    pub proposed: u64,
    pub propose_success: SyncSender<bool>,
    /// 0-none, 1-put, 2-get, 3-delete, 4-scan
    pub op_type: i64,
}

impl Proposal {
    /**
    Use for configure change, return a Proposal that contain changed config
    input: changed configure
    output: Configure change Proposal and Receiver
    **/
    pub fn conf_change(cc: &ConfChange) -> (Self, Receiver<bool>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: None,
            conf_change: Some(cc.clone()),
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
            op_type: 0,
        };
        (proposal, rx)
    }

    /**
    Use for normal Proposal, insert Key/Value pairs
    input: insert Key and Value
    output: Proposal and Receiver
    When the Proposal is applied, the Receiver will receive the Proposal
    **/
    pub fn normal(key: String, value: String, operation_type: i64) -> (Self, Receiver<bool>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: Some((key, value)),
            conf_change: None,
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
            op_type: operation_type,
        };
        (proposal, rx)
    }
}
