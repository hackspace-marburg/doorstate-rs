use std::fs;
use std::error::Error;
use std::path::Path;
use regex::Regex;
use chrono::prelude::*;
use chrono::Duration;
use spaceapi::Event;


pub fn next_events(dir: &Path) -> Result<Vec<Event>, Box<dyn Error>>{
    let now: DateTime<Local> = Local::now();
    let mut future_events = Vec::new();
    let reg = Regex::new(r"text=.+StartYear: ([0-9]+)%0aStartMonth: ([0-9]+)%0aStartDay: ([0-9]+)%0aStartTime: ([0-9]+):([0-9]+)%0a.+\n.+\ntitle=(.+)\n").unwrap();
    // TODO: optionale felder wie EndTime/Date oder EventDescription mit lesen
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let filename = entry.file_name().into_string().unwrap();
            if filename.contains("Event"){
                //print!("{}", filename);
                let file = fs::read_to_string(entry.path())?;
                let captures = reg.captures(file.as_str()).unwrap();
                let start = Local.ymd(
                    captures[1].parse::<i32>()?,
                    captures[2].parse::<u32>()?,
                    captures[3].parse::<u32>()?,
                ).and_hms(
                    captures[4].parse::<u32>()?,
                    captures[5].parse::<u32>()?,
                    0,
                );

                let event = Event{
                    name: String::from(&captures[6]),
                    timestamp: start.timestamp() as u64,
                    type_: String::from("Event"), // TODO: We don't differentiate im pmwiki
                    extra: None,
                };
                if start > now - Duration::days(1){
                    // Auch noch adden welche Events es gestern so gab. Zur Sicherheit.
                    // Für multi-day-events (Congress, rc3, Camp, etc.) doof. Dafür müsste das End Date noch beachtet werden, wenn da. TODO
                    future_events.push(event);
                }
            }
        }
        Ok(future_events)
    } else {
        Err(From::from("Path is not a directory"))
    }
    
}