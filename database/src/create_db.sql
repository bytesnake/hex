BEGIN;
    CREATE TABLE IF NOT EXISTS Tracks (
        Key         BLOB PRIMARY KEY, 
        Fingerprint BLOB NOT NULL, 
        Title       TEXT, 
        Album       TEXT, 
        Interpret   TEXT, 
        People      TEXT, 
        Composer    TEXT, 
        Duration    REAL NOT NULL, 
        FavsCount   INTEGER NOT NULL,
        Created     INTEGER NOT NULL

    );

    CREATE TABLE IF NOT EXISTS Playlists (
        Key     INTEGER PRIMARY KEY, 
        Title   TEXT NOT NULL, 
        Desc    TEXT, 
        Tracks  BLOB NOT NULL, 
        Author  BLOB NOT NULL
    );

    CREATE TABLE IF NOT EXISTS Tokens (
        Token       INTEGER PRIMARY KEY, 
        Key         INTEGER, 
        Played      BLOB NOT NULL, 
        Pos         NUMERIC, 
        LastUse     INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS Transitions (
        Key         BLOB Primary KEY,
        PublicKey   BLOB NOT NULL,
        Signature   BLOB NOT NULL,
        Refs        BLOB NOT NULL,
        State       INTEGER NOT NULL,
        Data        BLOB,
        Created     INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS Summary (
        Day         TEXT, 
        Transitions INTEGER,
        Tracks      INTEGER
    );
COMMIT;
