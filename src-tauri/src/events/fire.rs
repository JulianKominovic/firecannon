use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::Mutex;

use crate::utils::math::calculate_percentiles;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ResponseStats {
    pub status: u16,
    pub content_length: u64,
    pub content_type: String,
    pub headers: HashMap<String, String>,
    pub duration: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ResponseMetrics {
    pub duration: Vec<u128>,
    pub mean_duration: f64,
    pub median_duration: f64,
    pub min_duration: u128,
    pub max_duration: u128,
    pub duration_p_10: f64,
    pub duration_p_25: f64,
    pub duration_p_50: f64,
    pub duration_p_75: f64,
    pub duration_p_90: f64,
    pub duration_p_95: f64,
    pub duration_p_99: f64,
    pub total_redirects: u64,
    pub total_duration: u128,
    pub total_requests: u64,
    pub total_bytes: u64,
    pub total_errors: u64,
    pub responses: Vec<ResponseStats>,
}

async fn send_request(
    url: String,
    method: String,
    headers: HashMap<String, String>,
) -> Result<ResponseStats, String> {
    let client = reqwest::Client::new();
    println!("url {:?} method {:?}", url, method);

    let client = match method.as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        _ => {
            return Err(format!("Invalid method: {}", method).to_string());
        }
    };

    let mut client = client.header("user-agent", "the-awesome-agent/007");

    for (k, v) in headers.iter() {
        client = client.header(k, v);
    }
    let init_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let response = {
        let response = client.send().await;
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            - init_ms;
        match response {
            Ok(res) => {
                let status = res.status().as_u16();
                let mut headers = HashMap::new();
                let content_type = res
                    .headers()
                    .get("content-type")
                    .unwrap_or(&HeaderValue::from_str("").unwrap())
                    .to_str()
                    .unwrap_or_default()
                    .to_string();
                for (name, value) in res.headers().iter() {
                    headers.insert(
                        name.to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    );
                }
                let content_length = res.content_length().unwrap_or_else(|| {
                    // tauri::async_runtime::block_on(async { *&res.bytes().await.unwrap().len() })
                    //     .try_into()
                    //     .unwrap()
                    0
                });
                Ok(ResponseStats {
                    status,
                    content_length,
                    content_type,
                    headers,
                    duration,
                })
            }
            Err(e) => {
                println!("err {:?}", e);
                return Err(format!("Error: {}", e));
            }
        }
    };

    match response {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}

const UPDATE_TIME: u16 = 1000;

#[tauri::command]
#[specta::specta]
pub fn fire(
    window: tauri::Window,
    url: String,
    method: String,
    headers: HashMap<String, String>,
    parallel_requests: usize,
    duration_ms: u64,
) -> Result<(), String> {
    if parallel_requests == 0 {
        return Err("Parallel requests cannot be 0".to_string());
    }
    if parallel_requests > 1000 {
        return Err("Parallel requests cannot be greater than 1000".to_string());
    }
    if duration_ms <= 0 {
        return Err("Duration must be greater than 0 ms".to_string());
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let start_time = SystemTime::now();
    let metrics_vec: Arc<Mutex<Vec<ResponseMetrics>>> = Arc::new(Mutex::new(vec![]));
    let metrics_vec_cpy = metrics_vec.clone();

    rt.spawn(async move {
        while SystemTime::now()
            .duration_since(start_time)
            .expect("Failed to get system_time duration from start_time")
            .as_millis()
            < duration_ms as u128
        {
            let lock = metrics_vec_cpy.lock().await;
            window.emit("metrics_update", lock.clone()).unwrap();
            tokio::time::sleep(Duration::from_millis(UPDATE_TIME as u64)).await;
        }
    });

    while SystemTime::now()
        .duration_since(start_time)
        .expect("Failed to get system_time duration from start_time")
        .as_millis()
        < duration_ms as u128
    {
        let metrics = Arc::new(Mutex::new(ResponseMetrics {
            duration: vec![],
            mean_duration: 0.0,
            median_duration: 0.0,
            min_duration: 0,
            max_duration: 0,
            total_duration: 0,
            total_requests: 0,
            total_bytes: 0,
            total_errors: 0,
            total_redirects: 0,
            duration_p_10: 0.0,
            duration_p_25: 0.0,
            duration_p_50: 0.0,
            duration_p_75: 0.0,
            duration_p_90: 0.0,
            duration_p_95: 0.0,
            duration_p_99: 0.0,
            responses: vec![],
        }));
        let mut joins = vec![];

        rt.block_on(async {
            for _ in 0..parallel_requests {
                let url = url.clone();
                let method = method.clone();
                let headers = headers.clone();
                let binding = metrics.clone();
                let process_request = async move {
                    let mut metrics = binding.lock().await;
                    let response = send_request(url, method, headers).await;
                    match response {
                        Ok(res) => {
                            metrics.responses.push(res.clone());
                            metrics.duration.push(res.duration);
                            metrics.total_requests += 1;
                            metrics.total_duration += res.duration;
                            metrics.total_bytes += res.content_length;
                            if res.status >= 400 {
                                metrics.total_errors += 1;
                            }
                            if res.status >= 300 && res.status < 400 {
                                metrics.total_redirects += 1;
                            }
                        }
                        Err(e) => {
                            metrics.total_errors += 1;
                            println!("Error: {}", e);
                        }
                    }
                };
                joins.push(tokio::task::spawn(process_request));
            }
        });
        for join in joins {
            rt.block_on(join).unwrap();
        }

        let metrics = rt.block_on(async {
            let clone = metrics.clone();
            let metrics = clone.lock().await;
            metrics.clone()
        });
        let mean_duration = metrics.total_duration as f64 / metrics.total_requests as f64;
        let median_duration = {
            let mut duration = metrics.duration.clone();
            duration.sort();
            let len = duration.len();
            if len % 2 == 0 {
                (duration[len / 2] + duration[len / 2 - 1]) as f64 / 2.0
            } else {
                duration[len / 2] as f64
            }
        };
        let min_duration = *metrics.duration.iter().min().unwrap();
        let max_duration = *metrics.duration.iter().max().unwrap();
        let (
            duration_p_10,
            duration_p_25,
            duration_p_50,
            duration_p_75,
            duration_p_90,
            duration_p_95,
            duration_p_99,
        ) = calculate_percentiles(metrics.duration.clone());

        rt.block_on(async {
            let mut lock = metrics_vec.lock().await;
            lock.push(ResponseMetrics {
                mean_duration,
                median_duration,
                min_duration,
                max_duration,
                duration_p_10,
                duration_p_25,
                duration_p_50,
                duration_p_75,
                duration_p_90,
                duration_p_95,
                duration_p_99,
                ..metrics
            });
        });
    }

    Ok(())
}
