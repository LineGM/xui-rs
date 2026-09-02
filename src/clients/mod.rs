//! Client lifecycle, traffic, bulk operations, grouping, and attachment APIs.

mod api;
mod models;

#[cfg(test)]
mod contract_tests;

pub use api::ClientsApi;
pub use models::{
    ActiveInboundsByGuid, AffectedCount, BulkAdjustRequest, BulkAdjustResult, BulkAttachResult,
    BulkClientIssue, BulkCreateResult, BulkDeleteResult, BulkDetachResult, BulkFlowAdjustment,
    BulkSetEnabledResult, ClientConfig, ClientCreateRequest, ClientDetails, ClientExternalLink,
    ClientExternalLinkInput, ClientExternalLinkKind, ClientHwidDevice, ClientIpEntry, ClientIpInfo,
    ClientIpsByGuid, ClientMutationStatus, ClientPage, ClientPageRequest, ClientRecord,
    ClientReverse, ClientSlim, ClientSort, ClientStatusFilter, ClientSummary,
    ClientWithAttachments, ClientsByGuid, DeletedCount, GroupName, GroupSummary, LastOnlineByEmail,
    SortOrder,
};
