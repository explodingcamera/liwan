mod common;

#[tokio::test]
async fn it_adds_two() {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app, tx);
    let client = poem::test::TestClient::new(router);

    client.get("/").send().await;
}
