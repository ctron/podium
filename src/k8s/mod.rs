mod reflector;
mod scale;

use chrono::Utc;
use humantime::format_duration;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
pub use reflector::*;
pub use scale::*;

pub fn ago(time: &Time) -> Option<String> {
    let mut age: chrono::Duration = Utc::now() - time.0;

    if age > chrono::Duration::days(2) {
        // truncate to days
        age = chrono::Duration::days(age.num_days());
    }

    if age > chrono::Duration::hours(2) {
        // truncate to hours
        age = chrono::Duration::hours(age.num_hours());
    }

    if age > chrono::Duration::minutes(2) {
        age = chrono::Duration::minutes(age.num_minutes());
    }

    // finally, get rid of milliseconds
    age = chrono::Duration::seconds(age.num_seconds());

    let age = match age.to_std() {
        Ok(age) => age,
        Err(_) => return None,
    };

    Some(format_duration(age).to_string())
}
