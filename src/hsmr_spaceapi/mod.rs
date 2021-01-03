use serde::{Deserialize, Serialize};
use spaceapi::{Contact, IssueReportChannel, Location, Status, StatusBuilder};
use std::fs;
use std::path::Path;

mod wikiparse;

#[derive(Serialize, Deserialize)]
pub struct Tuerstatus {
    pub flti_only: Option<bool>,
    pub door_open: bool,
    pub timestamp: u64,
}

pub fn write_sitenav(wikipath: &Path, tuerstatus: &Tuerstatus) -> std::io::Result<()> {
    let sitenav = format!(
        concat!(
            "version=pmwiki-2.2.53 ordered=1 urlencoded=1\n",
            "name=Site.SiteNav\n",
            "targets=Infrastruktur.ServerB2s\n",
            "text=* [[#door]][[Infrastruktur/Door | %25black%25Base: <br />{}%25%25]]\n",
            "time={}"
        ),
        if tuerstatus.door_open {
            "%25green%25besetzt"
        } else {
            "%25red%25unbesetzt"
        },
        tuerstatus.timestamp
    );
    fs::write(
        format!(
            "{}{}",
            wikipath.as_os_str().to_str().unwrap(),
            "/wiki.d/Site.SiteNav"
        ),
        sitenav,
    )
}

pub fn write_spaceapi(wikipath: &Path, tuerstatus: &Tuerstatus) {
    let spaceapi = hsmr_spaceapi(wikipath, tuerstatus).expect("Spaceapi generation failed!");
    fs::write(
        format!(
            "{}{}",
            wikipath.as_os_str().to_str().unwrap(),
            "/spaceapi.json"
        ), // TODO: Make this less shit
        serde_json::to_string(&spaceapi).unwrap(),
    )
    .expect("Error writing spaceapi.json");
}

///  Generate spaceapi json with current events and given tuerstatus
fn hsmr_spaceapi(
    wikipath: &Path,
    tuerstatus: &Tuerstatus,
) -> Result<Status, Box<dyn std::error::Error>> {
    let mut base = hsmr_state_prefix();
    let events = wikiparse::next_events(wikipath)?;
    for event in events {
        base = base.add_event(event);
    }
    let mut built = base.build()?;
    built.state.open = Some(tuerstatus.door_open);
    built.state.lastchange = Some(tuerstatus.timestamp);
    Ok(built)
}

/// StatusBuilder containing basic HSMR Information which is (hopefully) not changing often
fn hsmr_state_prefix() -> StatusBuilder {
    StatusBuilder::new("[hsmr] - Hackspace Marburg")
        .logo("https://hsmr.cc/logo.svg")
        .url("https://hsmr.cc/")
        .location(Location {
            address: Some(
                "[hsmr] Hackspace Marburg, Rudolf-Bultmann-Strasse 2b, 35039 Marburg, Germany"
                    .into(),
            ),
            lat: 50.81615,
            lon: 8.77851,
        })
        .contact(Contact {
            email: Some("mail@hsmr.cc".into()),
            irc: Some("ircs://irc.hackint.org:6697/#hsmr".into()),
            ml: Some("public@lists.hsmr.cc".into()),
            phone: Some("+49 6421 4924981".into()),
            ..Default::default()
        })
        .add_issue_report_channel(IssueReportChannel::Email)
        .add_issue_report_channel(IssueReportChannel::Ml)
}
