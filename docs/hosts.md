# Subscription host overrides

`Client::hosts()` covers all 12 routes registered by the 3x-ui v3.6.0
`HostController`. Upstream OpenAPI contains 11; the source-defined
`/panel/api/hosts/bulk/add` alias is pinned by the SDK route snapshot as well.

Hosts are overrides used while generating subscription links and client
configuration. They are not DNS host records. A logical group can target
multiple inbounds and multiple public addresses. The panel expands a create or
update into one physical database row for every inbound/address combination.

| Operation | SDK method |
|---|---|
| List/get | `list`, `get`, `list_by_inbound`, `tags` |
| Create/replace | `create`, `bulk_create`, `update` |
| State/order | `set_enabled`, `reorder`, `bulk_set_enabled` |
| Delete | `delete`, `bulk_delete` |

## Group and row response shapes

List/get endpoints return `HostGroup`: a stable `group_id`, `inbound_ids`,
formatted `hosts`, and flattened `HostOptions`. Create and update return
`Vec<HostRow>` because the Go controller returns the physical rows it just
created. The upstream OpenAPI examples still show the older single-row shape;
the SDK follows the tagged controller and service implementation.

`list_by_inbound` also handles the source's empty-result `obj: null` response
and exposes it as an empty Rust vector.

## Creating and replacing groups

```rust,no_run
# use xui_rs::{Client, HostGroup, HostSecurity, VlessRoute};
# async fn example(client: &Client) -> xui_rs::Result<()> {
let mut group = HostGroup::new(vec![7, 9], "production CDN");
group.hosts = vec!["cdn.example.com".into(), "[2001:db8::10]:8443".into()];
group.options.port = 443;
group.options.security = HostSecurity::Tls;
group.options.sni = "origin.example.com".into();
group.options.vless_route = VlessRoute::new(443);

let rows = client.hosts().create(&group).await?;
println!("created {} physical rows", rows.len());
# Ok(())
# }
```

An empty `group_id` lets 3x-ui generate one. Supplying a non-empty value uses
that exact logical identifier. `update(group_id, group)` is a full replacement:
the service deletes the old rows and recreates the inbound/address Cartesian
product in one database transaction. Start from a fetched `HostGroup` when
editing an existing group.

Each `hosts` entry can include its own port. `HostOptions::port` is the fallback
for entries without one. IPv6 literals with an explicit port should use bracket
notation. Group IDs in URL paths are percent-encoded as one segment.

## Typed overrides

`HostSecurity`, `MihomoIpVersion`, and `SubscriptionFormat` expose the v3.6.0
vocabulary while retaining unknown strings from newer panels. `VlessRoute`
accepts only the source-supported 0–65535 range and maps its disabled state to
the historical empty-string wire representation.

3x-ui stores `muxParams`, `sockoptParams`, and `finalMask` as JSON strings.
`HostJsonOverride::from_value` and `value` handle that nested encoding so callers
do not stringify configuration manually. The raw representation remains
available for forward compatibility, while `Debug` only reports whether an
override is configured.

The API uses positive `enabled` arguments although the database stores the
inverse `isDisabled` flag. Empty bulk ID lists deliberately retain the upstream
no-op behavior.
