use std::io;

use social_service::{get_app_data, run_service, should_run_migration_helper};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_data = get_app_data(None).await;

    if should_run_migration_helper() {
        match app_data.db.run_helper().await {
            Ok(()) => log::info!("Migrations executed successfully."),
            Err(_) => log::info!("Error while executing the migrations"),
        }
        std::process::exit(0)
    }

    let server = run_service(app_data);
    if let Ok(server) = server {
        server.await?;
    }

    Ok(())
}
