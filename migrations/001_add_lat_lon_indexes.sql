-- Improve discovery bounding-box filters on latitude/longitude.
-- Composite index helps queries with both lat and lon BETWEEN clauses.
CREATE INDEX IF NOT EXISTS idx_users_lat_lon ON users(latitude, longitude);
