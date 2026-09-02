#![allow(missing_docs)]

use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{Client, SubscriptionBalancerInput, SubscriptionBalancerStrategy};

fn balancer_json() -> serde_json::Value {
    json!({
        "id": 7,
        "remark": "automatic",
        "strategy": "leastPing",
        "inboundIds": [1, 3],
        "sortOrder": 2,
        "enabled": true,
        "createdAt": 1_710_000_000_000_i64,
        "updatedAt": 1_710_000_001_000_i64
    })
}

#[tokio::test]
async fn every_subscription_balancer_route_uses_the_exact_form_contract() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/sub-balancers"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true, "msg": "", "obj": [balancer_json()]
        })))
        .expect(1)
        .mount(&server)
        .await;

    for path in ["/panel/api/sub-balancers", "/panel/api/sub-balancers/7"] {
        Mock::given(matchers::method("POST"))
            .and(matchers::path(path))
            .and(matchers::header("authorization", "Bearer api-secret"))
            .and(matchers::body_string_contains("remark=automatic"))
            .and(matchers::body_string_contains("strategy=leastPing"))
            .and(matchers::body_string_contains("sortOrder=2"))
            .and(matchers::body_string_contains("inboundIds=1"))
            .and(matchers::body_string_contains("inboundIds=3"))
            .and(matchers::body_string_contains("enabled=true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true, "msg": "", "obj": balancer_json()
            })))
            .expect(1)
            .mount(&server)
            .await;
    }

    for (method, path) in [
        ("DELETE", "/panel/api/sub-balancers/7"),
        ("POST", "/panel/api/sub-balancers/7/del"),
    ] {
        Mock::given(matchers::method(method))
            .and(matchers::path(path))
            .and(matchers::header("authorization", "Bearer api-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true, "msg": "", "obj": ""
            })))
            .expect(1)
            .mount(&server)
            .await;
    }

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let api = client.subscription_balancers();
    let listed = api.list().await.unwrap();
    assert_eq!(listed[0].strategy, SubscriptionBalancerStrategy::LeastPing);

    let mut input = SubscriptionBalancerInput::new("automatic", vec![1, 3]);
    input.strategy = SubscriptionBalancerStrategy::LeastPing;
    input.sort_order = 2;
    api.create(&input).await.unwrap();
    api.update(7, &input).await.unwrap();
    api.delete(7).await.unwrap();
    api.delete_via_post(7).await.unwrap();
}
