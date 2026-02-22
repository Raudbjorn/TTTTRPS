//! Cost Predictor Module
//!
//! Predicts future LLM costs based on usage patterns.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

// ============================================================================
// Types
// ============================================================================

/// Historical usage data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDataPoint {
    pub timestamp: DateTime<Utc>,
    pub cost: f64,
    pub tokens: u32,
    pub requests: u32,
}

/// Forecast horizon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForecastHorizon {
    Daily,
    Weekly,
    Monthly,
}

/// Usage pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsagePattern {
    /// Usage is relatively stable
    Stable,
    /// Usage is increasing
    Trending,
    /// Usage varies cyclically (e.g., weekday vs weekend)
    Cyclical,
    /// Not enough data to determine
    Unknown,
}

/// Cost forecast result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostForecast {
    /// Forecast horizon
    pub horizon: ForecastHorizon,
    /// Predicted cost
    pub predicted_cost: f64,
    /// Lower bound (95% confidence)
    pub lower_bound: f64,
    /// Upper bound (95% confidence)
    pub upper_bound: f64,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
    /// Detected usage pattern
    pub pattern: UsagePattern,
    /// Forecast generated at
    pub generated_at: DateTime<Utc>,
}

// ============================================================================
// Cost Predictor
// ============================================================================

/// Predicts future costs based on historical usage
pub struct CostPredictor {
    /// Historical data points (hourly aggregates)
    data_points: RwLock<VecDeque<UsageDataPoint>>,
    /// Maximum data points to keep (90 days of hourly data)
    max_data_points: usize,
}

impl CostPredictor {
    pub fn new() -> Self {
        Self {
            data_points: RwLock::new(VecDeque::with_capacity(2160)), // 90 days * 24 hours
            max_data_points: 2160,
        }
    }

    /// Record a usage data point
    pub fn record(&self, cost: f64, tokens: u32, requests: u32) {
        let mut data = self.data_points.write().unwrap();

        let now = Utc::now();

        // Try to aggregate with last data point if within same hour
        if let Some(last) = data.back_mut() {
            let hour_ago = now - Duration::hours(1);
            if last.timestamp > hour_ago {
                last.cost += cost;
                last.tokens += tokens;
                last.requests += requests;
                return;
            }
        }

        // Add new data point
        data.push_back(UsageDataPoint {
            timestamp: now,
            cost,
            tokens,
            requests,
        });

        // Rotate if needed
        while data.len() > self.max_data_points {
            data.pop_front();
        }
    }

    /// Generate a cost forecast
    pub fn forecast(&self, horizon: ForecastHorizon) -> CostForecast {
        let data = self.data_points.read().unwrap();

        // Determine how many hours to look back and forecast
        let (lookback_hours, forecast_hours) = match horizon {
            ForecastHorizon::Daily => (168, 24),    // 7 days back, 1 day forward
            ForecastHorizon::Weekly => (336, 168),  // 14 days back, 7 days forward
            ForecastHorizon::Monthly => (720, 720), // 30 days back, 30 days forward
        };

        // Get relevant data
        let cutoff = Utc::now() - Duration::hours(lookback_hours);
        let relevant: Vec<&UsageDataPoint> = data
            .iter()
            .filter(|d| d.timestamp > cutoff)
            .collect();

        if relevant.is_empty() {
            return CostForecast {
                horizon,
                predicted_cost: 0.0,
                lower_bound: 0.0,
                upper_bound: 0.0,
                confidence: 0.0,
                pattern: UsagePattern::Unknown,
                generated_at: Utc::now(),
            };
        }

        // Calculate statistics
        let total_cost: f64 = relevant.iter().map(|d| d.cost).sum();
        let avg_hourly_cost = total_cost / relevant.len() as f64;

        // Calculate variance for confidence interval
        let variance: f64 = relevant
            .iter()
            .map(|d| (d.cost - avg_hourly_cost).powi(2))
            .sum::<f64>()
            / relevant.len() as f64;
        let std_dev = variance.sqrt();

        // Detect usage pattern
        let pattern = self.detect_pattern(&relevant);

        // Calculate forecast
        let predicted_cost = avg_hourly_cost * forecast_hours as f64;

        // 95% confidence interval (approximately 2 standard deviations)
        let margin = 2.0 * std_dev * (forecast_hours as f64).sqrt();
        let lower_bound = (predicted_cost - margin).max(0.0);
        let upper_bound = predicted_cost + margin;

        // Confidence based on data availability
        let expected_points = lookback_hours as f64;
        let actual_points = relevant.len() as f64;
        let confidence = (actual_points / expected_points).min(1.0);

        CostForecast {
            horizon,
            predicted_cost,
            lower_bound,
            upper_bound,
            confidence,
            pattern,
            generated_at: Utc::now(),
        }
    }

    /// Detect usage pattern
    fn detect_pattern(&self, data: &[&UsageDataPoint]) -> UsagePattern {
        if data.len() < 24 {
            return UsagePattern::Unknown;
        }

        // Split data into first half and second half
        let mid = data.len() / 2;
        let first_half: f64 = data[..mid].iter().map(|d| d.cost).sum::<f64>() / mid as f64;
        let second_half: f64 = data[mid..].iter().map(|d| d.cost).sum::<f64>() / (data.len() - mid) as f64;

        // Check for trend
        let trend_threshold = 0.2; // 20% change
        let trend = (second_half - first_half) / first_half.max(0.01);

        if trend > trend_threshold {
            return UsagePattern::Trending;
        }

        // Check for cyclical pattern (compare weekdays)
        // Simplified: just check variance
        let avg: f64 = data.iter().map(|d| d.cost).sum::<f64>() / data.len() as f64;
        let variance: f64 = data
            .iter()
            .map(|d| (d.cost - avg).powi(2))
            .sum::<f64>()
            / data.len() as f64;
        let cv = variance.sqrt() / avg.max(0.01); // Coefficient of variation

        if cv > 0.5 {
            return UsagePattern::Cyclical;
        }

        UsagePattern::Stable
    }

    /// Get usage summary for a period
    pub fn get_summary(&self, hours: i64) -> UsageSummary {
        let data = self.data_points.read().unwrap();
        let cutoff = Utc::now() - Duration::hours(hours);

        let relevant: Vec<&UsageDataPoint> = data
            .iter()
            .filter(|d| d.timestamp > cutoff)
            .collect();

        if relevant.is_empty() {
            return UsageSummary::default();
        }

        let total_cost: f64 = relevant.iter().map(|d| d.cost).sum();
        let total_tokens: u32 = relevant.iter().map(|d| d.tokens).sum();
        let total_requests: u32 = relevant.iter().map(|d| d.requests).sum();

        let avg_cost_per_request = if total_requests > 0 {
            total_cost / total_requests as f64
        } else {
            0.0
        };

        UsageSummary {
            total_cost,
            total_tokens,
            total_requests,
            avg_cost_per_request,
            data_points: relevant.len(),
            period_hours: hours,
        }
    }

    /// Detect anomalies in recent usage
    pub fn detect_anomalies(&self) -> Vec<Anomaly> {
        let data = self.data_points.read().unwrap();
        let mut anomalies = Vec::new();

        if data.len() < 48 {
            return anomalies;
        }

        // Calculate baseline (excluding last 24 hours)
        let baseline_data: Vec<&UsageDataPoint> = data
            .iter()
            .rev()
            .skip(24)
            .take(168)
            .collect();

        if baseline_data.is_empty() {
            return anomalies;
        }

        let baseline_avg: f64 = baseline_data.iter().map(|d| d.cost).sum::<f64>()
            / baseline_data.len() as f64;
        let baseline_std: f64 = (baseline_data
            .iter()
            .map(|d| (d.cost - baseline_avg).powi(2))
            .sum::<f64>()
            / baseline_data.len() as f64)
            .sqrt();

        // Check recent data for anomalies
        let recent: Vec<&UsageDataPoint> = data.iter().rev().take(24).collect();
        let threshold = 3.0; // 3 standard deviations

        for point in recent {
            let z_score = (point.cost - baseline_avg) / baseline_std.max(0.01);
            if z_score.abs() > threshold {
                anomalies.push(Anomaly {
                    timestamp: point.timestamp,
                    cost: point.cost,
                    expected: baseline_avg,
                    z_score,
                    description: if z_score > 0.0 {
                        "Unusually high spending".to_string()
                    } else {
                        "Unusually low spending".to_string()
                    },
                });
            }
        }

        anomalies
    }
}

impl Default for CostPredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// Usage summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_cost: f64,
    pub total_tokens: u32,
    pub total_requests: u32,
    pub avg_cost_per_request: f64,
    pub data_points: usize,
    pub period_hours: i64,
}

/// Detected anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: DateTime<Utc>,
    pub cost: f64,
    pub expected: f64,
    pub z_score: f64,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording() {
        let predictor = CostPredictor::new();

        predictor.record(0.05, 100, 1);
        predictor.record(0.10, 200, 1);

        let summary = predictor.get_summary(24);
        assert!(summary.total_cost > 0.0);
    }

    #[test]
    fn test_forecast() {
        let predictor = CostPredictor::new();

        // Add some historical data
        for _ in 0..100 {
            predictor.record(0.05, 100, 1);
        }

        let forecast = predictor.forecast(ForecastHorizon::Daily);
        assert!(forecast.predicted_cost > 0.0);
        assert!(forecast.confidence > 0.0);
    }

    #[test]
    fn test_empty_forecast() {
        let predictor = CostPredictor::new();

        let forecast = predictor.forecast(ForecastHorizon::Daily);
        assert_eq!(forecast.predicted_cost, 0.0);
        assert_eq!(forecast.pattern, UsagePattern::Unknown);
    }
}
