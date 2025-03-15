-- Andy Coin D1 Database Schema

-- Guild statistics table
CREATE TABLE IF NOT EXISTS guild_stats (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  guild_id TEXT NOT NULL,
  user_count INTEGER NOT NULL,
  total_coins INTEGER NOT NULL,
  timestamp TEXT NOT NULL
);

-- Create index on guild_id
CREATE INDEX IF NOT EXISTS idx_guild_stats_guild_id ON guild_stats(guild_id);

-- Create index on timestamp
CREATE INDEX IF NOT EXISTS idx_guild_stats_timestamp ON guild_stats(timestamp);

-- Health check logs
CREATE TABLE IF NOT EXISTS health_checks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  status TEXT NOT NULL,
  status_code INTEGER,
  error_message TEXT,
  timestamp TEXT NOT NULL
);

-- Create index on status
CREATE INDEX IF NOT EXISTS idx_health_checks_status ON health_checks(status);

-- Create index on timestamp
CREATE INDEX IF NOT EXISTS idx_health_checks_timestamp ON health_checks(timestamp);
