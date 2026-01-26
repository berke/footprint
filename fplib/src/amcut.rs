#![allow(dead_code)]

use geo::{Polygon,LineString};
use geo_clipper::Clipper;

use crate::poly_utils;

pub fn segment_crosses_antimeridian((x0,_y0):(f64,f64),(x1,_y1):(f64,f64))->bool {
    let threshold = 180.0;
    fn f(x:f64)->f64 {
	(x + 180.0).rem_euclid(360.0) - 180.0
    }
    let x0 = f(x0);
    let x1 = f(x1);
    x0.signum() != x1.signum() && (x1 - x0).abs() >= threshold
}

pub fn crosses_antimeridian(ring:&[(f64,f64)])->bool {
    let m = ring.len();
    for i in 0..m {
	let iprev =
	    if i == 0 {
		m - 1
	    } else {
		i - 1
	    };
	if segment_crosses_antimeridian(ring[iprev],ring[i]) {
	    return true;
	}
    }
    false
}

pub fn cut_and_push(outline:&mut Vec<Vec<Vec<(f64,f64)>>>,ring:Vec<(f64,f64)>)->bool {
    if !crosses_antimeridian(&ring) {
	outline.push(vec![ring]);
	false
    } else {
	let ring_offset : Vec<(f64,f64)> = ring.iter().map(|&(x,y)| ((x + 360.0).rem_euclid(360.0),y)).collect();
	let ext : LineString<f64> = ring_offset.into();
	let poly = Polygon::new(ext,vec![]);

	let east_hemisphere = poly_utils::rectangle((0.0,-90.0),(180.0,90.0));
	let west_hemisphere = poly_utils::rectangle((180.0,-90.0),(360.0,90.0));

	let poly_east = east_hemisphere.intersection(&poly,poly_utils::FACTOR);
	let poly_west = west_hemisphere.intersection(&poly,poly_utils::FACTOR);

	let f_east = |(x,y)| if x > 180.0 { (x - 360.0,y) } else { (x,y) };
	let f_west = |(x,y)| if x >= 180.0 { (x - 360.0,y) } else { (x,y) };

	for p in poly_east.iter() {
	    outline.push(vec![poly_utils::ring_to_vec(p,f_east)]);
	}
	for p in poly_west.iter() {
	    outline.push(vec![poly_utils::ring_to_vec(p,f_west)]);
	}
	true
    }
}
