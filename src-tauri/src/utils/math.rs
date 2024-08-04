/// Calculate the percentile of a given data set.
/// 10, 25, 50, 75, 90, 95, 99 percentiles are calculated in this function.
pub fn calculate_percentiles(data: Vec<u128>) -> (f64, f64, f64, f64, f64, f64, f64) {
    if data.len() <= 0 {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    let len = data.len() - 1;
    let mut data = data;
    data.sort();
    (
        data[(len as f64 * 0.1).round() as usize] as f64,
        data[(len as f64 * 0.25).round() as usize] as f64,
        data[(len as f64 * 0.5).round() as usize] as f64,
        data[(len as f64 * 0.75).round() as usize] as f64,
        data[(len as f64 * 0.9).round() as usize] as f64,
        data[(len as f64 * 0.95).round() as usize] as f64,
        data[(len as f64 * 0.99).round() as usize] as f64,
    )
}
