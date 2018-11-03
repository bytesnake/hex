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
        Origin  TEXT
    );

    CREATE TABLE IF NOT EXISTS Tokens (
        Token       INTEGER PRIMARY KEY, 
        Key         INTEGER, 
        Played      BLOB NOT NULL, 
        Pos         NUMERIC, 
        Counter     INTEGER NOT NULL,
        LastUse  TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS Events (
        Date    Text, 
        Origin  Text, 
        Event   Text, 
        Data    TEXT
    );

    CREATE TABLE IF NOT EXISTS Summarise (
        Day         TEXT, 
        Connects    INTEGER, 
        Plays       INTEGER, 
        Adds        INTEGER, 
        Removes     INTEGER
    );
COMMIT;
