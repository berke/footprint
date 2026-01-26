#![allow(dead_code)]

mod stats;

use anyhow::{
    anyhow,
    bail,
    Context,
    Result,
};
use log::{trace,debug,warn,error,info};
use clap::{Arg,App};
use chrono::{Utc,TimeZone,NaiveDateTime,DateTime};
use footprint::{
    poly_utils,
    Footprint,
    Footprints
};
use stac::{
    Item
};
use stac_io::{
    FromJsonPath
};
use std::{
    fs::{
        File
    },
    io::{
        Read,
        BufReader,
        BufRead,
    },
    path::{
        Path
    }
};
use serde_json::{
    Value
};
use stats::Stats;

fn load_json<P:AsRef<Path>>(path:P)->Result<Value> {
    let mut fd = File::open(path)?;
    let mut u = String::new();
    fd.read_to_string(&mut u)?;
    let json = serde_json::from_str(&u)?;
    Ok(json)
}

fn outline_from_geojson_value(v:&geojson::Value)->Result<Vec<Vec<Vec<(f64,f64)>>>> {
    match v {
        geojson::Value::MultiPolygon(mp) => {
            Ok(mp.iter().map(|poly| {
                poly.iter().map(|ring| {
                    ring.iter().map(|pos| {
                        (pos[0],pos[1])
                    }).collect()
                }).collect()
            }).collect())
        },
        _ => bail!("Unsupported geometry type")
    }
}

fn f64_of_dt(dt:DateTime<Utc>)->f64 {
    dt.timestamp_millis() as f64/1e3
}

fn main()->Result<()> {
    let args = App::new("stac2fp")
        .arg(Arg::with_name("input").short("i")
            .takes_value(true)
            .value_name("INPUT")
            .help("STAC input file"))
        .arg(Arg::with_name("output")
            .short("o")
            .takes_value(true)
            .value_name("OUTPUT.mpk")
            .help("Footprint MPK output file"))
        .arg(Arg::with_name("verbose")
            .short("v")
            .help("Increase the detail level of printed messages"))
        .get_matches();

    let verbose = args.is_present("verbose");

    simple_logger::SimpleLogger::new()
        .with_level(
            if verbose { log::LevelFilter::Trace }
            else { log::LevelFilter::Info })
        .init()?;

    let input_path = args.value_of("input")
        .ok_or_else(|| anyhow!("Missing input path argument"))?;
    let output_path = args.value_of("output")
        .ok_or_else(|| anyhow!("Missing output path argument"))?;

    info!("Opening JSON file {:?}",input_path);

    let mut json : Value = load_json(input_path)?;

    let features : &mut Vec<Value> = json["features"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("features is not an array"))?;

    let mut footprints = Vec::new();

    let items : Result<Vec<Item>,_> =
        features.drain(..).map(|feat| serde_json::from_value(feat))
        .collect();
    let mut items = items?;

    let mut ts : Vec<f64> = items
        .iter()
        .filter_map(|it| it.properties.datetime.map(f64_of_dt))
        .collect();
    ts.sort_by(f64::total_cmp);

    let mut dt_est = None;

    // Figure out observation length
    let mut dt_stats = Stats::new();
    for tt in ts.windows(2) {
        if let [t1,t2] = tt {
            dt_stats.add(t2 - t1);
        }
    }

    const DT_THRESHOLD : f64 = 0.01;
    if dt_stats.count() > 0 {
        let (dt_min,dt_mean,dt_max) = dt_stats.summary();
        info!("Observation interval: {} {} {}",
            dt_min,
            dt_mean,
            dt_max);
        if (dt_max - dt_min) / dt_mean < DT_THRESHOLD {
            dt_est = Some(dt_mean);
        } else {
            warn!("Observation interval not good");
        }
    } else {
        warn!("Could not determine observation interval");
    }

    for it in items.drain(..) {
        if let Some(geo) = &it.geometry {
            // See if we have precise start and end times
            let t0_opt = it.properties.start_datetime.map(f64_of_dt);
            let t1_opt = it.properties.end_datetime.map(f64_of_dt);
            let (t0,t1) =
                if let (Some(t0),Some(t1)) =
                    (it.properties.start_datetime.map(f64_of_dt),
                    it.properties.end_datetime.map(f64_of_dt))
                {
                    (t0,t1)
                } else if let Some(t0) =
                    it.properties.datetime.map(f64_of_dt)
                {
                    (t0,t0 + dt_est.unwrap_or(0.0))
                } else {
                    warn!("Cannot get times");
                    (0.0,0.0)
                };

            let platform =
                it.properties.additional_fields.get("platform")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let orbit =
                it.properties.additional_fields
                .get("sat:absolute_orbit")
                .and_then(|p| p.as_i64())
                .map(|n| n.max(0) as usize)
                .unwrap_or(0);
            let instrument =
                it.properties.additional_fields
                .get("instruments")
                .and_then(|a| a.as_array())
                .map(|a| {
                    let mut u = String::new();
                    for k in a {
                        if let Some(v) = k.as_str() {
                            if !u.is_empty() {
                                u.push(',');
                            }
                            u.push_str(v);
                        }
                    };
                    u
                }).unwrap_or("".to_string());

            if verbose {
                info!("Props: {:#?}",it.properties);
            }
            let fp = Footprint {
                orbit,
                id:it.id,
                platform,
                instrument,
                time_interval:(t0,t1),
                outline:outline_from_geojson_value(&geo.value)?,
            };
            footprints.push(fp);
        } else {
            warn!("No geometry found");
        }
    }

    let fps = Footprints { footprints };
    fps.save_to_file(&output_path)?;

    Ok(())
}
