use std::io;

use social_service::{api::app::get_app_data, ws::app::run_ws_transport};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_data = get_app_data(None).await;

    // let server = run_service(app_data.clone());
    // if let Ok(server) = server {
    //     server.await?;
    // }

    let app_components = app_data.into_inner();
    run_ws_transport(app_components).await;

    Ok(())
}
