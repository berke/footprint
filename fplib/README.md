# footprint

This defines a simple format, serialized using MPK, containing
"footprints".  A footprint is simply a multipolygonal area given in
WGS84 coordinates, representing a remote sensing satellite's pixel.

Converting footprint information from different satellites to
a common format makes it more convenient when looking for simultaneous
(coincident) observations.

A tool, `fptool`, allows merging different files, exporting the
footprints in GeoJSON format (with or without pretty-printing),
drawing them as SVG or dumping their contexts in human-readable text
form.

# Changes

- 0.2.3: Update depdendencies
- 0.2: Use anyhow for errors, reorganize into workspace
- 0.1: Initial version
