DROP TABLE IF EXISTS instruments_fts;
DROP TABLE IF EXISTS instruments;

CREATE TABLE instruments (
  instrument_token INTEGER DEFAULT 0,
  trading_symbol TEXT NOT NULL,
  name           TEXT,
  exchange       TEXT,
  segment        TEXT,
  instrument_type TEXT,
  expiry         TEXT,
  last_price     REAL DEFAULT 0.0,
  lot_size       REAL,
  tick_size      REAL,
  updated_at     INTEGER NOT NULL
);

CREATE VIRTUAL TABLE instruments_fts USING fts5(
  trading_symbol,
  name,
  content='instruments',
  content_rowid='rowid'
);

CREATE TRIGGER instruments_ai AFTER INSERT ON instruments BEGIN
  INSERT INTO instruments_fts(rowid, trading_symbol, name)
  VALUES (new.rowid, new.trading_symbol, new.name);
END;

CREATE TRIGGER instruments_ad AFTER DELETE ON instruments BEGIN
  INSERT INTO instruments_fts(instruments_fts, rowid, trading_symbol, name)
  VALUES ('delete', old.rowid, old.trading_symbol, old.name);
END;
