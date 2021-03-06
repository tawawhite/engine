use crate::cloud_provider::digitalocean::api_structs::clusters::Clusters;
use crate::container_registry::docr::get_header_with_bearer;
use crate::error::{SimpleError, SimpleErrorKind};
use crate::object_storage::do_space::download_space_object;
use reqwest::StatusCode;
use std::fs::File;
use std::io::Write;

extern crate serde_json;

pub fn kubernetes_config_path(
    workspace_directory: &str,
    kubernetes_cluster_id: &str,
    region: &str,
    spaces_secret_key: &str,
    spaces_access_id: &str,
) -> Result<String, SimpleError> {
    let kubernetes_config_bucket_name = format!("qovery-kubeconfigs-{}", kubernetes_cluster_id);
    let kubernetes_config_object_key = format!("{}.yaml", kubernetes_cluster_id);

    let kubernetes_config_file_path = format!(
        "{}/kubernetes_config_{}",
        workspace_directory, kubernetes_cluster_id
    );

    let kubeconfig = download_space_object(
        spaces_access_id,
        spaces_secret_key,
        kubernetes_config_bucket_name.as_str(),
        kubernetes_config_object_key.as_str(),
        region,
    );
    match kubeconfig {
        Ok(body) => {
            let mut file =
                File::create(kubernetes_config_file_path.clone()).expect("unable to create file");
            file.write_all(body.as_bytes()).expect("unable to write");
            Ok(kubernetes_config_file_path)
        }
        Err(e) => Err(e),
    }
}

pub const do_cluster_api_path: &str = "https://api.digitalocean.com/v2/kubernetes/clusters";

/*
Waiting for https://github.com/pandaman64/serde-query/issues/2
#[derive(serde_query::Deserialize)]
struct Cluster {
    #[query(r#".["kubernetes_clusters"].id"#)]
    cluster_id: String,
    #[query(r#".["kubernetes_clusters"].name"#)]
    cluster_name: String,
}
*/

pub fn get_uuid_of_cluster(token: &str, kubeID: &str) -> Result<String, SimpleError> {
    let mut headers = get_header_with_bearer(token);
    let res = reqwest::blocking::Client::new()
        .get(do_cluster_api_path)
        .headers(headers)
        .send();
    match res {
        Ok(response) => match response.status() {
            StatusCode::OK => {
                let content = response.text().unwrap();
                let res_clusters  = serde_json::from_str::<Clusters>(&content);
                match res_clusters{
                    Ok(clusters) => match search_uuid_cluster_for(kubeID,clusters){
                        Some(uuid) => return Ok(uuid),
                        None => return Err(SimpleError::new(
                            SimpleErrorKind::Other,
                            Some(
                                "Unable to retrieve cluster id from this name",
                            ),
                        ))
                    }
                    Err(e) => {
                        print!("{}", e);
                        return Err(SimpleError::new(
                            SimpleErrorKind::Other,
                            Some(
                                "While trying to deserialize json received from Digital Ocean API",
                            ),
                        ));
                    },
                }
            }
            _ => return Err(SimpleError::new(
                SimpleErrorKind::Other,
                Some(
                    "Receive weird status Code from Digital Ocean while retrieving the cluster list",
                ),
            )),
        },
        Err(_) => {
            return Err(SimpleError::new(
                SimpleErrorKind::Other,
                Some("Unable to get any responses from Digital Ocean"),
            ))
        }
    }
}

fn search_uuid_cluster_for(kubeName: &str, clusters: Clusters) -> Option<String> {
    for cluster in clusters.kubernetes_clusters {
        match cluster.name.eq(kubeName) {
            true => return Some(cluster.id),
            _ => {}
        }
    }
    None
}
