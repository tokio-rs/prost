//! ===========================================================================
//!  Consensus Metadata
//! ===========================================================================

/// A peer in a configuration.
#[derive(Debug, Message)]
pub struct RaftPeerPB {
    /// Permanent uuid is optional: RaftPeerPB/RaftConfigPB instances may
    /// be created before the permanent uuid is known (e.g., when
    /// manually specifying a configuration for Master/CatalogManager);
    /// permament uuid can be retrieved at a later time through RPC.
    #[proto(tag="1")]
    pub permanent_uuid: Vec<u8>,
    #[proto(tag="2")]
    pub member_type: kudu::consensus::raft_peer_pb::MemberType,
    #[proto(tag="3")]
    pub last_known_addr: Option<kudu::HostPortPB>,
}
mod raft_peer_pb {
    /// The possible roles for peers.
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum Role {
        UnknownRole = 999,
        /// Indicates this node is a follower in the configuration, i.e. that it participates
        /// in majorities and accepts Consensus::Update() calls.
        Follower = 0,
        /// Indicates this node is the current leader of the configuration, i.e. that it
        /// participates in majorities and accepts Consensus::Append() calls.
        Leader = 1,
        /// Indicates that this node participates in the configuration in a passive role,
        /// i.e. that it accepts Consensus::Update() calls but does not participate
        /// in elections or majorities.
        Learner = 2,
        /// Indicates that this node is not a participant of the configuration, i.e. does
        /// not accept Consensus::Update() or Consensus::Update() and cannot
        /// participate in elections or majorities. This is usually the role of a node
        /// that leaves the configuration.
        NonParticipant = 3,
    }
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum MemberType {
        UnknownMemberType = 999,
        NonVoter = 0,
        Voter = 1,
    }
}
/// A set of peers, serving a single tablet.
#[derive(Debug, Message)]
pub struct RaftConfigPB {
    /// The index of the operation which serialized this RaftConfigPB through
    /// consensus. It is set when the operation is consensus-committed (replicated
    /// to a majority of voters) and before the consensus metadata is updated.
    /// It is left undefined if the operation isn't committed.
    #[proto(tag="1")]
    pub opid_index: i64,
    /// Obsolete. This parameter has been retired.
    #[proto(tag="2")]
    pub OBSOLETE_local: bool,
    /// The set of peers in the configuration.
    #[proto(tag="3")]
    pub peers: Vec<kudu::consensus::RaftPeerPB>,
}
/// Represents a snapshot of a configuration at a given moment in time.
#[derive(Debug, Message)]
pub struct ConsensusStatePB {
    /// A configuration is always guaranteed to have a known term.
    #[proto(tag="1")]
    pub current_term: i64,
    /// There may not always be a leader of a configuration at any given time.
    ///
    /// The node that the local peer considers to be leader changes based on rules
    /// defined in the Raft specification. Roughly, this corresponds either to
    /// being elected leader (in the case that the local peer is the leader), or
    /// when an update is accepted from another node, which basically just amounts
    /// to a term check on the UpdateConsensus() RPC request.
    ///
    /// Whenever the local peer sees a new term, the leader flag is cleared until
    /// a new leader is acknowledged based on the above critera. Simply casting a
    /// vote for a peer is not sufficient to assume that that peer has won the
    /// election, so we do not update this field based on our vote.
    ///
    /// The leader listed here, if any, should always be a member of 'configuration', and
    /// the term that the node is leader of _must_ equal the term listed above in
    /// the 'current_term' field. The Master will use the combination of current
    /// term and leader uuid to determine when to update its cache of the current
    /// leader for client lookup purposes.
    ///
    /// There is a corner case in Raft where a node may be elected leader of a
    /// pending (uncommitted) configuration. In such a case, if the leader of the pending
    /// configuration is not a member of the committed configuration, and it is the committed
    /// configuration that is being reported, then the leader_uuid field should be
    /// cleared by the process filling in the ConsensusStatePB object.
    #[proto(tag="2")]
    pub leader_uuid: String,
    /// The peers. In some contexts, this will be the committed configuration,
    /// which will always have configuration.opid_index set. In other contexts, this may
    /// a "pending" configuration, which is active but in the process of being committed.
    /// In any case, initial peership is set on tablet start, so this
    /// field should always be present.
    #[proto(tag="3")]
    pub config: Option<kudu::consensus::RaftConfigPB>,
}
/// This PB is used to serialize all of the persistent state needed for
/// Consensus that is not in the WAL, such as leader election and
/// communication on startup.
#[derive(Debug, Message)]
pub struct ConsensusMetadataPB {
    /// Last-committed peership.
    #[proto(tag="1")]
    pub committed_config: Option<kudu::consensus::RaftConfigPB>,
    /// Latest term this server has seen.
    /// When a configuration is first created, initialized to 0.
    ///
    /// Whenever a new election is started, the candidate increments this by one
    /// and requests votes from peers.
    ///
    /// If any RPC or RPC response is received from another node containing a term higher
    /// than this one, the server should step down to FOLLOWER and set its current_term to
    /// match the caller's term.
    ///
    /// If a follower receives an UpdateConsensus RPC with a term lower than this
    /// term, then that implies that the RPC is coming from a former LEADER who has
    /// not realized yet that its term is over. In that case, we will reject the
    /// UpdateConsensus() call with ConsensusErrorPB::INVALID_TERM.
    ///
    /// If a follower receives a RequestConsensusVote() RPC with an earlier term,
    /// the vote is denied.
    #[proto(tag="2")]
    pub current_term: i64,
    /// Permanent UUID of the candidate voted for in 'current_term', or not present
    /// if no vote was made in the current term.
    #[proto(tag="3")]
    pub voted_for: String,
}
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum ConsensusConfigType {
    ConsensusConfigUnknown = 999,
    /// Committed consensus config. This includes the consensus configuration that
    /// has been serialized through consensus and committed, thus having a valid
    /// opid_index field set.
    ConsensusConfigCommitted = 1,
    /// Active consensus config. This could be a pending consensus config that
    /// has not yet been committed. If the config is not committed, its opid_index
    /// field will not be set.
    ConsensusConfigActive = 2,
}
/// An id for a generic state machine operation. Composed of the leaders' term
/// plus the index of the operation in that term, e.g., the <index>th operation
/// of the <term>th leader.
#[derive(Debug, Message)]
pub struct OpId {
    /// The term of an operation or the leader's sequence id.
    #[proto(tag="1")]
    pub term: i64,
    #[proto(tag="2")]
    pub index: i64,
}
