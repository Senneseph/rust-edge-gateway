-- Pet Store Schema
-- Compatible with SQLite, PostgreSQL, and MySQL

-- Pets table
CREATE TABLE IF NOT EXISTS pets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT,
    tags TEXT,  -- JSON array stored as string
    status TEXT NOT NULL DEFAULT 'available',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create index on status for filtering
CREATE INDEX IF NOT EXISTS idx_pets_status ON pets(status);

-- Orders table (for future expansion)
CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    pet_id TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'placed',
    ship_date TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (pet_id) REFERENCES pets(id)
);

-- Sample data
INSERT OR IGNORE INTO pets (id, name, category, tags, status, created_at, updated_at) VALUES
    ('pet-001', 'Buddy', 'dog', '["friendly", "trained"]', 'available', datetime('now'), datetime('now')),
    ('pet-002', 'Whiskers', 'cat', '["playful"]', 'available', datetime('now'), datetime('now')),
    ('pet-003', 'Goldie', 'fish', '["low-maintenance"]', 'sold', datetime('now'), datetime('now'));

