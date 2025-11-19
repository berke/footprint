use anyhow::Result;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader,BufWriter,Write};
use serde::{Serialize,Deserialize};
use chrono::{Utc,TimeZone};
use geojson::{Feature,FeatureCollection,Geometry,Value};
use serde_json::{Map,to_value};

pub use geo;

pub mod amcut;
pub mod minisvg;
pub mod poly_utils;

use minisvg::MiniSVG;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Footprint {
    pub orbit:usize,
    pub id:String,
    pub platform:String,
    pub instrument:String,
    pub time_interval:(f64,f64),
    pub outline:Vec<Vec<Vec<(f64,f64)>>> // (longitude,latitude) pairs
}

pub trait FootprintLike {
    fn orbit(&self)->usize;
    fn id(&self)->&str;
    fn platform(&self)->&str;
    fn instrument(&self)->&str;
    fn time_interval(&self)->(f64,f64);
    fn outline(&self)->&Vec<Vec<Vec<(f64,f64)>>>;
    fn mean_time(&self)->f64 {
	let ti = self.time_interval();
	(ti.0 + ti.1) / 2.0
    }
    fn properties(&self)->Map<String,serde_json::Value> {
	Map::new()
    }
}

impl FootprintLike for Footprint {
    fn orbit(&self)->usize { self.orbit }
    fn id(&self)->&str { &self.id }
    fn platform(&self)->&str { &self.platform }
    fn instrument(&self)->&str { &self.instrument }
    fn time_interval(&self)->(f64,f64) { self.time_interval }
    fn outline(&self)->&Vec<Vec<Vec<(f64,f64)>>> { &self.outline }
}

impl Footprint {
    pub fn new()->Self {
	Self{
	    orbit:0,
	    id:String::new(),
	    platform:String::new(),
	    instrument:String::new(),
	    time_interval:(0.0,0.0),
	    outline:Vec::new()
	}
    }

    pub fn min_coords(&self)->(f64,f64) {
	self.outline.iter().fold(
	    (std::f64::INFINITY,std::f64::INFINITY),
	    |curr,poly|
	    poly.iter().fold(curr,
			     |curr2,ring|
			     ring.iter().fold(
				 curr2,
				 |(lon0,lat0),&(lon,lat)| (lon0.min(lon),lat0.min(lat)))))
    }

    pub fn max_coords(&self)->(f64,f64) {
	self.outline.iter().fold(
	    (std::f64::NEG_INFINITY,std::f64::NEG_INFINITY),
	    |curr,poly|
	    poly.iter().fold(curr,
			     |curr2,ring|
			     ring.iter().fold(
				 curr2,
				 |(lon0,lat0),&(lon,lat)| (lon0.max(lon),lat0.max(lat)))))
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Footprints {
    pub footprints:Vec<Footprint>
}

impl Footprints {
    pub fn new()->Self {
	Self {
	    footprints:Vec::new()
	}
    }

    pub fn from_file<P:AsRef<Path>>(path:P)->Result<Self> {
	let fd = File::open(path)?;
	let mut buf = BufReader::new(fd);
	let fps : Self = rmp_serde::decode::from_read(&mut buf)?;
	Ok(fps)
    }

    pub fn save_to_file<P:AsRef<Path>>(&self,path:P)->Result<()> {
	let fd = File::create(path)?;
	let mut buf = BufWriter::new(fd);
	self.serialize(&mut rmp_serde::Serializer::new(&mut buf))?;
	Ok(())
    }

    pub fn draw<P:AsRef<Path>>(&self,path:P)->Result<()> {
	let Footprints{ footprints } = self;
	let mut ms = MiniSVG::new(path,360.0,180.0,-180.0,-90.0)?;
	for fp in footprints.iter() {
	    ms.set_stroke(Some((0xff0000,0.01,1.0)));
	    ms.set_fill(Some((0xffff80,0.25)));
	    ms.multi_polygon(&fp.outline)?;
	    let (lon0,lat0) = fp.max_coords();
	    let t = fp.mean_time();
	    let ts =
		Utc.timestamp_opt(
		    t.floor() as i64,
		    (t.fract() * 1e9 + 0.5).floor() as u32)
		.unwrap();
	    let tss = ts.format("%H:%M");
	    ms.set_fill(Some((0x000000,1.00)));
	    ms.text(lon0,lat0,0.2,&tss.to_string())?;
	}
	Ok(())
    }

    pub fn dump_to_file<P:AsRef<Path>>(&self,
				       path:P)->Result<()> {
	dump_to_file(&self.footprints,path)
    }

    pub fn export_geojson<P:AsRef<Path>>(&self,
					 pretty:bool,
					 path:P)->Result<()> {
	export_geojson(&self.footprints,pretty,path)
    }
}

pub fn dump_to_file<P:AsRef<Path>>(footprints:&[Footprint],
				   path:P)->Result<()> {
    let fd = File::create(path)?;
    let mut buf = BufWriter::new(fd);

    for (ifp,fp) in footprints.iter().enumerate() {
	let Footprint {
	    orbit,
	    id,
	    platform,
	    instrument,
	    time_interval:(t0,t1),
	    outline
	} = fp;
	writeln!(buf,"Footprint {}",ifp)?;
	writeln!(buf,"  ID {}",id)?;
	writeln!(buf,"  Orbit {}",orbit)?;
	writeln!(buf,"  Platform {}",platform)?;
	writeln!(buf,"  Instrument {}",instrument)?;
	writeln!(buf,"  Start time {}",t0)?;
	writeln!(buf,"  End time {}",t1)?;
	let npoly = outline.len();
	writeln!(buf,"  Number of polygons: {}",npoly)?;
	for (ipoly,poly) in outline.iter().enumerate() {
	    let nring = poly.len();
	    writeln!(buf,"  Polygon {}",ipoly)?;
	    writeln!(buf,"    Number of rings: {}",nring)?;
	    for (iring,ring) in poly.iter().enumerate() {
		let nvert = ring.len();
		writeln!(buf,"    Ring {} of polygon {}",iring,ipoly)?;
		writeln!(buf,"      Number of vertices: {}",nvert)?;
		writeln!(buf,"        {:4} {:4} {:4} {:10} {:10}","Poly","Ring","Vert","Lon","Lat")?;
		for (ivert,vert) in ring.iter().enumerate() {
		    writeln!(buf,"        {:4} {:4} {:4} {:+10.3} {:+10.3}",
			     ipoly,
			     iring,
			     ivert,
			     vert.0,
			     vert.1)?;
		}
		writeln!(buf,"    End of ring {} of polygon {}",iring,ipoly)?;
	    }
	    writeln!(buf,"  End of polygon {}",ipoly)?;
	}
	writeln!(buf,"End of footprint {}",ifp)?;
    }
    Ok(())
}

pub fn export_geojson<P:AsRef<Path>,F:FootprintLike>(
    footprints:&[F],
    pretty:bool,
    path:P)->Result<()> {
	let fd = File::create(path)?;
	let mut buf = BufWriter::new(fd);

	let mut features = Vec::new();
	for fp in footprints.iter() {
	    let t = fp.mean_time();
	    let ts = Utc.timestamp_opt(
		t.floor() as i64,
		(t.fract() * 1e9 + 0.5).floor() as u32)
		.unwrap();
	    let tss = ts.to_string();

	    let mut gjmpoly : Vec<Vec<Vec<Vec<f64>>>> = Vec::new();
	    for poly in fp.outline().iter() {
		let mut gjpoly : Vec<Vec<Vec<f64>>> = Vec::new();
		for ring in poly.iter() {
		    let mut gjring : Vec<Vec<f64>> = 
			ring
			.iter()
			.map(|&(x,y)| vec![x,y])
			.collect();
		    if let Some(p) = gjring.last() {
			gjring.push(p.clone());
		    }
		    gjpoly.push(gjring);
		}
		gjmpoly.push(gjpoly);
	    }

	    let properties = {
		let mut props = fp.properties();
		props.insert(
		    String::from("time"),
		    to_value(tss.to_string()).unwrap());
		props.insert(
		    String::from("id"),
		    to_value(fp.id()).unwrap());
		Some(props)
	    };
	    let geo = Geometry::new(Value::MultiPolygon(gjmpoly));
	    let feature = Feature {
		bbox:None,
		geometry:Some(geo),
		id:None,
		properties,
		foreign_members:None
	    };
	    features.push(feature);
	}
	let fc = FeatureCollection {
	    bbox:None,
	    features,
	    foreign_members:None
	};
	if pretty {
	    serde_json::ser::to_writer_pretty(&mut buf,&fc)?;
	} else {
	    serde_json::ser::to_writer(&mut buf,&fc)?;
	}
	Ok(())
    }
