mod route;
use route::publish_routes;

use crate::commands::subdomain::Subdomain;
use crate::http;
use crate::settings::global_user::GlobalUser;
use crate::settings::toml::{DeployConfig, Zoneless};

pub fn worker(
    user: &GlobalUser,
    deploy_config: &DeployConfig,
) -> Result<Vec<String>, failure::Error> {
    match deploy_config {
        DeployConfig::Zoneless(zoneless_config) => {
            // this is a zoneless deploy
            log::info!("publishing to workers.dev subdomain");
            let deploy_address = publish_zoneless(user, zoneless_config)?;
            let addresses = vec![deploy_address];
            Ok(addresses)
        }
        DeployConfig::Zoned(zoned_config) => {
            // this is a zoned deploy
            log::info!("publishing to zone {}", zoned_config.zone_id);

            let published_routes = publish_routes(&user, zoned_config)?;

            let addresses: Vec<String> =
                published_routes.iter().map(|r| format!("{}", r)).collect();

            Ok(addresses)
        }
    }
}

fn publish_zoneless(
    user: &GlobalUser,
    zoneless_config: &Zoneless,
) -> Result<String, failure::Error> {
    log::info!("checking that subdomain is registered");
    let subdomain = match Subdomain::get(&zoneless_config.account_id, user)? {
        Some(subdomain) => subdomain,
        None => failure::bail!("Before publishing to workers.dev, you must register a subdomain. Please choose a name for your subdomain and run `wrangler subdomain <name>`.")
    };

    let sd_worker_addr = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/{}/subdomain",
        zoneless_config.account_id, zoneless_config.script_name,
    );

    let client = http::legacy_auth_client(user);

    log::info!("Making public on subdomain...");
    let res = client
        .post(&sd_worker_addr)
        .header("Content-type", "application/json")
        .body(build_subdomain_request())
        .send()?;

    if !res.status().is_success() {
        failure::bail!(
            "Something went wrong! Status: {}, Details {}",
            res.status(),
            res.text()?
        )
    }

    Ok(format!(
        "https://{}.{}.workers.dev",
        zoneless_config.script_name, subdomain
    ))
}

fn build_subdomain_request() -> String {
    serde_json::json!({ "enabled": true }).to_string()
}
