use rusqlite::{params, Connection, Result};

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("poker_users.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS statistics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            game_number INTEGER,
            total_bets INTEGER,
            total_winnings INTEGER,
            FOREIGN KEY (username) REFERENCES users(username) ON DELETE CASCADE
        )",
        [],
    )?;
   
    Ok(conn)
}
pub fn register_user(conn: &Connection, username: &str, password: &str) -> Result<bool> {
    let result = conn.execute(
        "INSERT INTO users (username, password) VALUES (?1, ?2)",
        params![username, password],
    );
    Ok(result.is_ok())
}

pub fn login_user(conn: &Connection, username: &str, password: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE username = ?1 AND password = ?2")?;
    let count: i64 = stmt.query_row(params![username, password], |row| row.get(0))?;
    Ok(count == 1)
}


pub fn insert_game_stats(conn: &Connection, username: &str, game_number: u32, total_bets: u32, total_winnings: u32) -> Result<()> {
    conn.execute(
        "INSERT INTO statistics (username, game_number, total_bets, total_winnings)
         VALUES (?1, ?2, ?3, ?4)",  
        params![username, game_number, total_bets, total_winnings],
    )?;
    Ok(())
}


pub fn fetch_stats_by_user(conn: &Connection, username: &str) -> Result<Vec<(u32, u32, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT game_number, total_bets, total_winnings FROM statistics WHERE username = ?1"
    )?;
    let stats = stmt.query_map([username], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(stats)
}
