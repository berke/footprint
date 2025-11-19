#![allow(dead_code)]

use geo::{MultiPolygon,Polygon,LineString};
use geo::algorithm::intersects::Intersects;
use geo_clipper::Clipper;

pub const FACTOR : f64 = (1 << 24) as f64 / 360.0;

pub fn ring_to_vec<F:Fn((f64,f64))->(f64,f64)>(p:&Polygon<f64>,f:F)->Vec<(f64,f64)> {
    let (pve1,_) = p.clone().into_inner();
    let pve1 : Vec<(f64,f64)> = pve1.points().map(|pt| f((pt.x(),pt.y()))).collect();
    pve1
}

pub fn polygon_to_vec<F:Fn((f64,f64))->(f64,f64)>(p:&Polygon<f64>,f:F)->Vec<Vec<(f64,f64)>> {
    let (pve1,pvi1) = p.clone().into_inner();
    let pve1 : Vec<(f64,f64)> = pve1.points().map(|pt| f((pt.x(),pt.y()))).collect();
    let mut pvi1 : Vec<Vec<(f64,f64)>> =
	pvi1.iter().map(|ls| ls.points().map(|pt| f((pt.x(),pt.y()))).collect()).collect();
    let mut u = Vec::new();
    u.push(pve1);
    u.append(&mut pvi1);
    u
}

pub fn multipolygon_to_vec(mp:&MultiPolygon<f64>)->Vec<Vec<Vec<(f64,f64)>>> {
    mp.iter().map(|p| polygon_to_vec(p,|q| q)).collect()
}

pub fn clip_to_roi(roi:&Polygon<f64>,mp:&MultiPolygon<f64>)->Option<MultiPolygon<f64>> {
    let mut res = Vec::new();
    for p in mp.iter() {
	if roi.intersects(p) {
	    let inter = roi.intersection(p,FACTOR);
	    let mut inter : Vec<Polygon<f64>> = inter.iter().map(|x| x.clone()).collect();
	    res.append(&mut inter);
	}
    }
    if res.len() > 0 {
	let mp_out : MultiPolygon<f64> = res.into();
	Some(mp_out)
    } else {
	None
    }
}

pub fn rectangle((lon0,lat0):(f64,f64),(lon1,lat1):(f64,f64))->Polygon<f64> {
    Polygon::new(
	LineString::from(vec![
	    (lon0,lat0),
	    (lon1,lat0),
	    (lon1,lat1),
	    (lon0,lat1)
	]),
	vec![])
}

pub fn ring_to_polygon(ring:&Vec<(f64,f64)>)->Polygon<f64> {
    let exterior : LineString<f64> = ring.clone().into();
    Polygon::new(exterior,vec![])
}

pub fn outline_to_multipolygon(outline:&Vec<Vec<Vec<(f64,f64)>>>)->MultiPolygon<f64> {
    let mut u = Vec::new();
    // f.outline: Vec<Vec<Vec<(f64,f64)>>>
    // f.outline.iter(): &Vec<Vec<(f64,f64)>>
    // f.outline.iter().iter(): &Vec<(f64,f64)>
    for a in outline.iter() {
	let m = a.len();
	if m > 0 {
	    let exterior : LineString<f64> = a[0].clone().into();
	    let interior : Vec<LineString<f64>> = a.iter().skip(1).map(|o| {
		let ls : LineString<f64> = o.clone().into();
		ls
	    }).collect();
	    let poly = Polygon::new(exterior,interior);
	    u.push(poly);
	}
    }
    MultiPolygon::from(u)
}

pub fn outline_points(outline:&Vec<Vec<Vec<(f64,f64)>>>)->Vec<(f64,f64)> {
    let mut pts = Vec::new();
    for poly in outline.iter() {
	for ring in poly.iter() {
	    for &pt in ring.iter() {
		pts.push(pt);
	    }
	}
    }
    pts
}

pub fn bounding_box(pts:&[(f64,f64)])->((f64,f64),(f64,f64)) {
    let mut x0 = f64::INFINITY;
    let mut x1 = f64::NEG_INFINITY;
    let mut y0 = f64::INFINITY;
    let mut y1 = f64::NEG_INFINITY;
    for &(x,y) in pts {
	x0 = x0.min(x);
	x1 = x1.max(x);
	y0 = y0.min(y);
	y1 = y1.max(y);
    }
    ((x0,x1),(y0,y1))
}
