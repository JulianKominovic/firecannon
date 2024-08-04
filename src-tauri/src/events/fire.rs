use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::{
    runtime::Builder,
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

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
    pub requests_per_second: f64,
    pub total_requests: u64,
    pub total_bytes: u64,
    pub total_errors: u64,
    pub timestamp: u128,
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
pub async fn fire(
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
    if parallel_requests > 5000 {
        return Err(
            "Easy cowboy 🔫 parallel requests cannot exceed 5000 or your PC will catch on 🔥"
                .to_string(),
        );
    }
    if duration_ms <= 0 {
        return Err("Duration must be greater than 0 ms".to_string());
    }
    if duration_ms < UPDATE_TIME.into() {
        return Err(format!("Duration must be greater than {} ms", UPDATE_TIME).to_string());
    }

    let events_rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut handles: Vec<JoinHandle<()>> = vec![];
    let start_time = SystemTime::now();
    let metrics = Arc::new(Mutex::new(ResponseMetrics {
        duration: vec![],
        mean_duration: 0.0,
        median_duration: 0.0,
        min_duration: 0,
        max_duration: 0,
        total_duration: 0,
        requests_per_second: 0.0,
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
        timestamp: start_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        responses: vec![],
    }));
    let (send, mut recv) = mpsc::channel::<u8>(1);
    let metrics_cpy = metrics.clone();

    let events_broadcaster = events_rt.spawn(async move {
        while SystemTime::now()
            .duration_since(start_time)
            .expect("Failed to get system_time duration from start_time")
            .as_millis()
            < duration_ms as u128
        {
            tokio::time::sleep(Duration::from_millis(UPDATE_TIME as u64)).await;

            let lock = metrics_cpy.lock().await;

            let mean_duration = lock.total_duration as f64 / lock.total_requests as f64;
            let median_duration = {
                let mut duration = lock.duration.clone();
                duration.sort();
                let len = duration.len();
                if len == 0 {
                    0.0
                } else if len == 1 {
                    duration[0] as f64
                } else if len % 2 == 0 {
                    (duration[len / 2] + duration[len / 2 - 1]) as f64 / 2.0
                } else {
                    duration[len / 2] as f64
                }
            };
            let min_duration = *lock.duration.iter().min().unwrap_or(&(0 as u128));
            let max_duration = *lock.duration.iter().max().unwrap_or(&(0 as u128));
            let (
                duration_p_10,
                duration_p_25,
                duration_p_50,
                duration_p_75,
                duration_p_90,
                duration_p_95,
                duration_p_99,
            ) = calculate_percentiles(lock.duration.clone());
            let requests_per_second = lock.total_requests as f64
                / (SystemTime::now()
                    .duration_since(start_time)
                    .expect("Failed to get system_time duration from start_time")
                    .as_secs_f64()
                    + 0.0001);
            let timestamp: u128 = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            window
                .emit(
                    "metrics_update",
                    ResponseMetrics {
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
                        requests_per_second,
                        timestamp,
                        ..lock.clone()
                    },
                )
                .unwrap();
        }
    });

    let mut initial_connections = 0;
    while SystemTime::now()
        .duration_since(start_time)
        .expect("Failed to get system_time duration from start_time")
        .as_millis()
        < duration_ms as u128
        && (initial_connections < parallel_requests || recv.recv().await.is_some())
    {
        initial_connections += 1;
        let sender_clone = send.clone();
        let url = url.clone();
        let method = method.clone();
        let headers = headers.clone();
        let binding = metrics.clone();
        handles.push(tokio::task::spawn(async move {
            let response = send_request(url, method, headers).await;
            let mut metrics = binding.lock().await;
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
            sender_clone.send(1).await.expect("Failed to send message");
        }))
    }

    for handle in handles {
        handle.await.expect("Failed to await handle");
    }
    events_broadcaster
        .await
        .expect("Failed to await events broadcaster");

    Ok(())
}
