#![allow(dead_code)]

mod stats;

use anyhow::{
    anyhow,
    Context,
    Result,
};
use log::{trace,info};
use clap::{Arg,App};
use chrono::{Utc,TimeZone,NaiveDateTime,DateTime};
use footprint::Footprints;
use stats::Stats;

fn timestamp_from_str(u:&str)->Result<f64> {
    let ndt : NaiveDateTime =
        NaiveDateTime::parse_from_str(u,"%Y-%m-%dT%H:%M:%S")?;
    let ts : DateTime<_> = Utc.from_utc_datetime(&ndt);
    Ok(ts.timestamp_millis() as f64 / 1000.0)
}

fn main()->Result<()> {
    let args = App::new("fptool")
	.arg(Arg::with_name("input").multiple(true))
	.arg(Arg::with_name("concat").short("c")
	     .value_name("OUTPUT.mpk")
	     .takes_value(true)
	     .help("Concatenate multiple input MPK files"))
	.arg(Arg::with_name("draw").short("d")
	     .takes_value(true)
	     .value_name("OUTPUT.svg")
	     .help("Render footprints to an SVG file"))
	.arg(Arg::with_name("export").short("e")
	     .takes_value(true)
	     .value_name("OUTPUT.geojson")
	     .help("Export footprints as GeoJSON"))
	.arg(Arg::with_name("dump").short("D")
	     .value_name("OUTPUT.txt")
	     .takes_value(true)
	     .help("Dump contents as an indented text file"))
	.arg(Arg::with_name("verbose").short("v")
	     .help("Increase the detail level of printed messages"))
	.arg(Arg::with_name("pretty").short("p")
	     .help("Pretty-print the JSON output"))
	.arg(Arg::with_name("t_min").long("t-min")
	     .value_name("Y-m-dTH:M:S")
	     .help("Start of time range")
	     .takes_value(true))
	.arg(Arg::with_name("t_max").long("t-max")
	     .value_name("Y-m-dTH:M:S")
	     .help("End of time range")
	     .takes_value(true))
	.arg(Arg::with_name("decimate").long("decimate")
	     .value_name("N")
	     .help("Keep only every Nth footprint")
	     .default_value("1").takes_value(true))
	.get_matches();

    let verbose = args.is_present("verbose");
    let pretty = args.is_present("pretty");

    simple_logger::SimpleLogger::new()
	.with_level(if verbose { log::LevelFilter::Trace } else { log::LevelFilter::Info })
	.init()?;

    let mut footprints = Vec::new();

    let mut lat_stats = Stats::new();
    let mut lon_stats = Stats::new();

    let t_min =
        args.value_of("t_min").map(timestamp_from_str).transpose()?
        .unwrap_or(0.0);
    let t_max =
        args.value_of("t_max").map(timestamp_from_str).transpose()?
        .unwrap_or(std::f64::INFINITY);

    let decimate : usize = args.value_of("decimate")
	.unwrap()
	.parse()
	.context("Invalid decimation value")?;

    let fp_fns = args.values_of("input")
	.ok_or_else(|| anyhow!("Specify footprint files"))?;
    let mut n = 0;
    for fp_fn in fp_fns {
	info!("Footprint file {}",fp_fn);
	let fps = Footprints::from_file(fp_fn)?;
	let m = fps.footprints.len();
	info!("Number of footprints: {}",m);
	for i in 0..m {
	    let fp = &fps.footprints[i];
	    let (t0,t1) = fp.time_interval;
	    if !(t_min <= t0 && t1 < t_max) {
		continue;
	    }
	    let skip = n % decimate != 0;
	    n += 1;
	    if skip {
		continue;
	    }
	    let ts0 = Utc.timestamp_opt(
		t0.floor() as i64,
		(t0.fract() * 1e9 + 0.5).floor() as u32)
		.unwrap();
	    let ts1 = Utc.timestamp_opt(
		t1.floor() as i64,
		(t1.fract() * 1e9 + 0.5).floor() as u32)
		.unwrap();
	    trace!("Time: {} to {}",ts0,ts1);
	    trace!("Orbit: {}",fp.orbit);
	    trace!("Platform: {}",fp.platform);
	    trace!("Instrument: {}",fp.instrument);
	    trace!("ID: {}",fp.id);
	    for poly in fp.outline.iter() {
		for ring in poly.iter() {
		    for &(lon,lat) in ring.iter() {
			lon_stats.add(lon);
			lat_stats.add(lat);
		    }
		}
	    }
	    footprints.push(fp.clone());
	}
    }
    let (lon0,lon_mean,lon1) = lon_stats.summary();
    let (lat0,lat_mean,lat1) = lat_stats.summary();
    info!("Longitude range: {} to {}, mean {}",lon0,lon1,lon_mean);
    info!("Latitude range: {} to {}, mean {}",lat0,lat1,lat_mean);

    let fps = Footprints{ footprints };
    if let Some(draw_fn) = args.value_of("draw") {
	fps.draw(draw_fn)?;
    }

    if let Some(dump_fn) = args.value_of("dump") {
	info!("Dumping footprint information to text file {}",dump_fn);
	fps.dump_to_file(dump_fn)?;
    }

    if let Some(path) = args.value_of("concat") {
	let m = fps.footprints.len();
	info!("Saving {} footprints to {}",m,path);
	fps.save_to_file(path)?;
    }

    if let Some(export_fn) = args.value_of("export") {
	info!("Exporting as GeoJSON to {}, pretty printing: {}",
	      export_fn,
	      pretty);
	fps.export_geojson(pretty,export_fn)?;
    }

    Ok(())
}
