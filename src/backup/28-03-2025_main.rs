#[macro_use]
extern crate serde_json;

use std::env;
use rocket::form::FromForm;

use reqwest::Client;
use scraper::{ Html, Selector };
use std::error::Error;

use rocket::{
    get,
    post,
    routes,
    serde::{ json::Json, Deserialize, Serialize },
    State,
    patch,
    http::Status,
    fairing::{ Fairing, Info },
};
use serde_json::Value; // Import Value from serde_json
use serde_json::json; // Import the json! macro

use sqlx::{ mysql::MySqlPoolOptions, FromRow, MySql, Pool, Row };

use std::fmt;

#[derive(Debug)]
struct ValidationError {
    message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

impl From<ValidationError> for sqlx::Error {
    fn from(err: ValidationError) -> Self {
        sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
    }
}

#[derive(Deserialize, Serialize, FromRow)]
#[serde(crate = "rocket::serde")]
struct Song {
    uuid: String,
    song: String,
    cover_url: String,
    song_id: Option<String>,
    playertype: Option<String>,
    artist: Option<String>,
    title: Option<String>,
    requester: Option<String>,
}

#[derive(FromRow, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueSong {
    Queueid: Option<i32>,
    Uuid: Option<String>,
    Trackid: String,
    Artist: String,
    Title: String,
    Length: String,
    Requester: String,
    Played: i32,
    Albumcover: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueuePostPayload {
    queueItem: QueueSong,
    uuid: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct SongPayload {
    uuid: String,
    key: String,
    song: String,
    cover: Option<String>,
    song_id: Option<String>,
    playertype: Option<String>,
    artist: Option<String>,
    title: Option<String>,
    requester: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueUpdatePayload {
    queueid: i32,
    uuid: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueClearPayload {
    uuid: String,
    key: String,
}

#[derive(FromRow)]
struct Usage {
    UUID: String,
    tst: String,
    twitch_id: i32,
    twitch_name: String,
    vs: Option<String>,
    playertype: Option<String>,
    access_key: Option<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Telemetry {
    uuid: String,
    key: String,
    tst: i32,
    twitch_id: String,
    twitch_name: String,
    vs: Option<String>,
    playertype: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct HistoryPayload {
    id: String,
    song: String,
    key: String,
    tst: i32,
}
#[derive(Deserialize, Serialize, FromRow)]
#[serde(crate = "rocket::serde")]
struct History {
    uuid: String,
    song: String,
    tst: String,
}

#[derive(Serialize, Deserialize, FromRow)]
#[serde(crate = "rocket::serde")]
struct Motd {
    Id: i32,
    MessageText: String,
    Severity: String,
    CreatedAt: i64,
    StartDate: Option<i64>,
    EndDate: Option<i64>,
    IsActive: bool,
    Author: String,
}

#[derive(FromForm)]
struct QueueParams {
    uuid: Option<String>,
    name: Option<String>,
    full: Option<bool>,
}

pub enum QueueParam {
    Id(String),
    Name(String),
}

struct Cors;

fn get_custom_uuids(uuid: &str) -> &str {
    match uuid {
        "inzaniity" => "43efb299-2504-4365-8ac6-a301f0d7c7aa",
        "thejaydizzle" => "5d07c1d6-6dcc-4185-a6bd-284fe0480b79",
        "sluckz" => "f6d9a390-7d48-4da6-a177-c378a7a33c1e",
        "vigilsc" => "07632164-719f-43ee-87eb-a1c9b4991506",
        "itsbustre" => "c90b6e0e-6706-4036-bf25-327b2d981082",
        "rocketstarrl" => "de8a9f85-2919-474c-9845-6534ec54dc7f",
        "preheet" => "4aa39d0a-1bf6-4705-bfb5-512dd8afc1e2",
        "highitsky" => "630e6596-a833-42d9-a905-7a5bf1a75d0e",
        _ => uuid,
    }
}

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: rocket::fairing::Kind::Response,
        }
    }

    async fn on_response<'r>(
        &self,
        _request: &'r rocket::Request<'_>,
        response: &mut rocket::Response<'r>
    ) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(
            rocket::http::Header::new(
                "Access-Control-Allow-Methods",
                "POST, GET, OPTIONS, PATCH, DELETE"
            )
        );
        response.set_header(
            rocket::http::Header::new("Access-Control-Allow-Headers", "Content-Type")
        );
    }
}

impl History {
    pub async fn set_history(history: Self, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx
            ::query("INSERT INTO songify_history (uuid, song, tst) VALUES (?, ?, ?)")
            .bind(history.uuid)
            .bind(history.song)
            .bind(history.tst)
            .execute(pool).await?;

        Ok(())
    }

    pub async fn get_history(id: String, pool: &Pool<MySql>) -> Result<Vec<Self>, sqlx::Error> {
        let uuid = get_custom_uuids(&id);
        let history = sqlx
            ::query_as::<MySql, Self>(
                "SELECT * FROM songify_history WHERE uuid = ? ORDER BY tst DESC"
            )
            .bind(uuid)
            .fetch_all(pool).await?;

        Ok(history)
    }
}

impl Motd {
    pub async fn get_active_motds(pool: &Pool<MySql>) -> Result<Vec<Self>, sqlx::Error> {
        let motds = sqlx
            ::query_as::<MySql, Self>(
                "SELECT Id, MessageText, Severity, CreatedAt, StartDate, EndDate, IsActive, Author FROM MotdMessages WHERE IsActive = 1 ORDER BY CreatedAt DESC"
            )
            .fetch_all(pool).await?;

        Ok(motds)
    }

    pub async fn get_all_motds(pool: &Pool<MySql>) -> Result<Vec<Self>, sqlx::Error> {
        let motds = sqlx
            ::query_as::<MySql, Self>(
                "SELECT Id, MessageText, Severity, CreatedAt, StartDate, EndDate, IsActive, Author FROM MotdMessages ORDER BY CreatedAt DESC"
            )
            .fetch_all(pool).await?;

        Ok(motds)
    }
}

impl Song {
    pub async fn set_song(song: Self, pool: &Pool<MySql>) -> sqlx::Result<()> {
        let result = sqlx
            ::query(
                "REPLACE INTO song_data 
            (UUID, song, cover_url, song_id, playertype, artist, title, requester) 
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&song.uuid)
            .bind(&song.song)
            .bind(&song.cover_url)
            .bind(&song.song_id)
            .bind(&song.playertype)
            .bind(song.artist.as_deref())
            .bind(song.title.as_deref())
            .bind(song.requester.as_deref())
            .execute(pool).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("❌ SQL Error: {}", e); // Log any SQL error
                Err(e)
            }
        }
    }

    pub async fn get_song(param: QueueParam, pool: &Pool<MySql>) -> Result<Self, sqlx::Error> {
        // Capture the `id` or `name` before the match expression
        let (id_or_name, query) = match param {
            QueueParam::Id(id) =>
                (
                    id.clone(), // Clone the id for use later
                    sqlx
                        ::query_as::<MySql, Self>("SELECT * FROM song_data WHERE uuid = ?")
                        .bind(id),
                ),
            QueueParam::Name(name) =>
                (
                    name.clone(), // Clone the name for use later
                    sqlx
                        ::query_as::<MySql, Self>(
                            "SELECT sd.*
                     FROM song_data sd
                     JOIN (
                         SELECT UUID
                         FROM songify_usage
                         WHERE LOWER(twitch_name) = LOWER(?)
                         ORDER BY tst DESC
                         LIMIT 1
                     ) su ON sd.uuid = su.UUID;"
                        )
                        .bind(name),
                ),
        };

        let song_result = query.fetch_one(pool).await;

        match song_result {
            Ok(song) => Ok(song), // If a song is found, return it
            Err(_) =>
                Ok(Self {
                    uuid: id_or_name, // Now you can use the captured id or name
                    song: "No song found".to_string(),
                    cover_url: String::new(),
                    song_id: None,
                    playertype: None,
                    artist: None,
                    title: None,
                    requester: None,
                }),
        }
    }
}

impl QueueSong {
    pub async fn get_queue(
        param: QueueParam,
        pool: &Pool<MySql>
    ) -> Result<Vec<Self>, sqlx::Error> {
        let query = match param {
            QueueParam::Id(id) => {
                sqlx::query_as::<MySql, Self>(
                    "SELECT * FROM songify_queue WHERE Uuid = ? AND Played = 0;"
                ).bind(id)
            }
            QueueParam::Name(name) => {
                sqlx::query_as::<MySql, Self>(
                    "
                SELECT sq.*
                FROM songify_queue sq
                JOIN (
                    SELECT UUID
                    FROM songify_usage
                    WHERE LOWER(twitch_name) = LOWER(?)
                    ORDER BY tst DESC
                    LIMIT 1
                ) su ON sq.Uuid = su.UUID
                WHERE sq.played = 0
                ORDER BY sq.Queueid ASC;"
                ).bind(name)
            }
        };

        let queue = query.fetch_all(pool).await?;

        Ok(queue)
    }

    pub async fn add_to_queue(id: String, song: Self, pool: &Pool<MySql>) -> sqlx::Result<Self> {
        use sqlx::Row;

        let inserted_song = sqlx
            ::query(
                "INSERT INTO songify_queue (Queueid, Uuid, Trackid, Artist, Title, Length, Requester, Played, Albumcover) VALUES (NULL, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *"
            )
            .bind(id)
            .bind(&song.Trackid)
            .bind(&song.Artist)
            .bind(&song.Title)
            .bind(&song.Length)
            .bind(&song.Requester)
            .bind(0)
            .bind(&song.Albumcover)
            .fetch_one(pool).await?;

        Ok(Self {
            Queueid: inserted_song.get(0),
            Uuid: inserted_song.get(1),
            Trackid: inserted_song.get(2),
            Artist: inserted_song.get(3),
            Title: inserted_song.get(4),
            Length: inserted_song.get(5),
            Requester: inserted_song.get(6),
            Played: inserted_song.get(7),
            Albumcover: inserted_song.get(8),
        })
    }

    pub async fn remove_from_queue(
        uuid: String,
        queueid: i32,
        pool: &Pool<MySql>
    ) -> sqlx::Result<()> {
        sqlx
            ::query("UPDATE songify_queue SET Played = 1 WHERE Uuid = ? AND Queueid = ?")
            .bind(uuid)
            .bind(queueid)
            .execute(pool).await?;

        Ok(())
    }

    pub async fn clear_queue(uuid: String, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx
            ::query("UPDATE songify_queue SET Played = 1 WHERE Uuid = ?")
            .bind(uuid)
            .execute(pool).await?;

        Ok(())
    }
}

impl Usage {
    pub async fn get_access_key(
        id: String,
        pool: &Pool<MySql>
    ) -> Result<Option<String>, sqlx::Error> {
        let usage = sqlx
            ::query("SELECT access_key FROM songify_usage WHERE UUID = ?")
            .bind(id)
            .fetch_one(pool).await;

        match usage {
            Ok(usage) => Ok(Some(usage.get(0))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => { Err(e) }
        }
    }

    pub async fn set_access_key(
        id: String,
        access_key: String,
        pool: &Pool<MySql>
    ) -> sqlx::Result<()> {
        sqlx
            ::query("UPDATE songify_usage SET access_key = ? WHERE UUID = ?")
            .bind(access_key)
            .bind(id)
            .execute(pool).await?;

        Ok(())
    }

    pub async fn set_telemetry(telemetry: Telemetry, pool: &Pool<MySql>) -> sqlx::Result<()> {
        if telemetry.uuid.is_empty() {
            return Err(
                (ValidationError {
                    message: "UUID cannot be empty".to_string(),
                }).into()
            );
        }
        sqlx
            ::query(
                "REPLACE INTO songify_usage (UUID, tst, twitch_id, twitch_name, vs, playertype, access_key) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(telemetry.uuid)
            .bind(telemetry.tst.to_string())
            .bind(telemetry.twitch_id)
            .bind(telemetry.twitch_name)
            .bind(telemetry.vs)
            .bind(telemetry.playertype)
            .bind(telemetry.key)
            .execute(pool).await?;

        Ok(())
    }

    pub async fn get_twitch_name(
        id: String,
        pool: &Pool<MySql>
    ) -> Result<Option<String>, sqlx::Error> {
        let usage = sqlx
            ::query("SELECT twitch_name FROM songify_usage WHERE UUID = ?")
            .bind(id)
            .fetch_one(pool).await;

        match usage {
            Ok(usage) => Ok(Some(usage.get(0))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => { Err(e) }
        }
    }
}

async fn verify_access_key(
    uuid: &str,
    api_key: &str,
    pool: &State<Pool<MySql>>
) -> Result<(), Status> {
    let access_key = Usage::get_access_key(uuid.to_string(), pool).await.map_err(
        |_| Status::InternalServerError
    )?;

    match access_key {
        Some(key) => {
            if key != api_key {
                println!("Access key mismatch");
                println!("{} != {}", key, api_key);
                return Err(Status::Unauthorized);
            }
        }
        None => {
            Usage::set_access_key(uuid.to_string(), api_key.to_string(), pool).await.map_err(
                |_| Status::InternalServerError
            )?;
        }
    }

    Ok(())
}

#[get("/getsong?<params..>")]
async fn get_song(pool: &State<Pool<MySql>>, params: QueueParams) -> Result<Json<Value>, Status> {
    let param = if let Some(uuid) = params.uuid {
        QueueParam::Id(uuid)
    } else if let Some(name) = params.name {
        QueueParam::Name(name)
    } else {
        return Err(Status::BadRequest);
    };

    let song = Song::get_song(param, pool).await.map_err(|_| Status::InternalServerError)?;

    // Check the "full" parameter; if true, return the full object
    if params.full.unwrap_or(false) {
        Ok(Json(json!(song))) // Return the full song object as JSON
    } else {
        Ok(content::Plain(song.song))
    }
}

#[get("/getcover?<params..>")]
async fn get_cover(pool: &State<Pool<MySql>>, params: QueueParams) -> Result<String, Status> {
    let param = if let Some(uuid) = params.uuid {
        QueueParam::Id(uuid)
    } else if let Some(name) = params.name {
        QueueParam::Name(name)
    } else {
        return Err(Status::BadRequest);
    };

    Song::get_song(param, pool).await.map_or(Err(Status::InternalServerError), |song|
        Ok(song.cover_url)
    )
}

#[get("/queue?<params..>")]
async fn get_queue(
    pool: &State<Pool<MySql>>,
    params: QueueParams
) -> Result<Json<Vec<QueueSong>>, Status> {
    let param = if let Some(uuid) = params.uuid {
        QueueParam::Id(uuid)
    } else if let Some(name) = params.name {
        QueueParam::Name(name)
    } else {
        return Err(Status::BadRequest);
    };

    QueueSong::get_queue(param, pool).await.map_or(Err(Status::InternalServerError), |queue|
        Ok(Json(queue))
    )
}

#[post("/queue?<api_key>", format = "json", data = "<song>")]
async fn add_to_queue(
    pool: &State<Pool<MySql>>,
    api_key: &str,
    song: Json<QueuePostPayload>
) -> Result<Json<QueueSong>, Status> {
    let song = song.into_inner();
    verify_access_key(&song.uuid, api_key, pool).await?;

    QueueSong::add_to_queue(song.uuid, song.queueItem, pool).await.map_or(
        Err(Status::InternalServerError),
        |song| Ok(Json(song))
    )
}

#[patch("/queue?<api_key>", format = "json", data = "<song>")]
async fn set_queue_song_played(
    pool: &State<Pool<MySql>>,
    api_key: &str,
    song: Json<QueueUpdatePayload>
) -> Result<(), Status> {
    let song = song.into_inner();
    verify_access_key(&song.uuid, api_key, pool).await?;

    match QueueSong::remove_from_queue(song.uuid, song.queueid, pool).await {
        Ok(_) => (),
        Err(_) => {
            return Err(Status::InternalServerError);
        }
    }

    Ok(())
}

#[post("/queue_delete?<api_key>", format = "json", data = "<queue>")]
async fn clear_queue(
    pool: &State<Pool<MySql>>,
    api_key: &str,
    queue: Json<QueueClearPayload>
) -> Result<(), Status> {
    let queue = queue.into_inner();
    verify_access_key(&queue.uuid, api_key, pool).await?;

    match QueueSong::clear_queue(queue.uuid, pool).await {
        Ok(_) => (),
        Err(_) => {
            return Err(Status::InternalServerError);
        }
    }

    Ok(())
}

#[post("/telemetry", format = "json", data = "<telemetry>")]
async fn set_telemetry(
    pool: &State<Pool<MySql>>,
    telemetry: Json<Telemetry>
) -> Result<(), Status> {
    let data = telemetry.into_inner();
    verify_access_key(&data.uuid, &data.key, pool).await?;
    Usage::set_telemetry(data, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[post("/song?<api_key>", format = "json", data = "<song>")]
async fn set_song(
    pool: &State<Pool<MySql>>,
    api_key: String,
    song: Json<SongPayload>
) -> Result<(), Status> {
    let data = song.into_inner();

    let cover = data.cover.map_or_else(String::new, |cover| cover);

    let song: Song = Song {
        uuid: data.uuid,
        song: data.song,
        cover_url: cover,
        song_id: data.song_id,
        playertype: data.playertype,
        artist: data.artist,
        title: data.title,
        requester: data.requester,
    };

    verify_access_key(&song.uuid, &data.key, pool).await?;
    Song::set_song(song, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[post("/history?<api_key>", format = "json", data = "<payload>")]
async fn set_history(
    pool: &State<Pool<MySql>>,
    api_key: String,
    payload: Json<HistoryPayload>
) -> Result<(), Status> {
    let payload = payload.into_inner();
    verify_access_key(&payload.id, &api_key, pool).await?;

    let history = History {
        uuid: payload.id,
        song: payload.song,
        tst: payload.tst.to_string(),
    };

    History::set_history(history, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[get("/motd")]
async fn motd(pool: &State<Pool<MySql>>) -> Result<Json<Vec<Motd>>, Status> {
    match Motd::get_active_motds(pool).await {
        Ok(motds) => Ok(Json(motds)),
        Err(e) => {
            eprintln!("Error fetching MOTD: {:?}", e); // Log the error
            Err(Status::InternalServerError)
        }
    }
}

#[get("/motd_all")]
async fn motd_all(pool: &State<Pool<MySql>>) -> Result<Json<Vec<Motd>>, Status> {
    match Motd::get_all_motds(pool).await {
        Ok(motds) => Ok(Json(motds)),
        Err(e) => {
            eprintln!("Error fetching MOTD: {:?}", e); // Log the error
            Err(Status::InternalServerError)
        }
    }
}

#[get("/history_data?<id>")]
async fn get_history_data(
    pool: &State<Pool<MySql>>,
    id: String
) -> Result<Json<Vec<History>>, Status> {
    History::get_history(id, pool).await.map_or(Err(Status::InternalServerError), |history|
        Ok(Json(history))
    )
}

#[get("/twitch_name?<id>")]
async fn get_twitch_name(pool: &State<Pool<MySql>>, id: String) -> Result<String, Status> {
    Usage::get_twitch_name(id, pool).await.map_or(Err(Status::InternalServerError), |name|
        Ok(match name {
            Some(name) => name,
            None => String::new(),
        })
    )
}

#[get("/canvas/<id>")]
async fn get_canvas(
    id: String,
    db_pool: &State<Pool<MySql>>, // Use Pool<MySql> here
    client: &State<Client>
) -> Result<Json<String>, String> {
    // Check if the canvas URL is cached in the database
    let cached_canvas: Option<String> = sqlx
        ::query_scalar("SELECT canvas_url FROM canvas_cache WHERE track_id = ?")
        .bind(&id)
        .fetch_optional(db_pool.inner()).await
        .map_err(|err| err.to_string())?;

    if let Some(canvas_url) = cached_canvas {
        return Ok(Json(canvas_url));
    }

    // Build the URL to query the local /canvas endpoint
    let url = format!("http://localhost:3020/canvas?id={}", id);

    let response = client
        .get(&url)
        .send().await
        .map_err(|err| err.to_string())?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await.map_err(|err| err.to_string())?;
        if let Some(canvas_url) = json["canvasUrl"].as_str() {
            let canvas_url = canvas_url.to_string();

            // Cache the result in DB
            sqlx
                ::query(
                    "INSERT INTO canvas_cache (track_id, canvas_url) VALUES (?, ?) 
                 ON DUPLICATE KEY UPDATE canvas_url = VALUES(canvas_url), cached_at = CURRENT_TIMESTAMP"
                )
                .bind(&id)
                .bind(&canvas_url)
                .execute(db_pool.inner()).await
                .map_err(|err| err.to_string())?;

            return Ok(Json(canvas_url));
        }
    }

    Err("No canvas found".to_string())
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let database_url = env::var("DATABASE_URL").map_or_else(
        |_| {
            println!("No database url found");
            std::process::exit(1);
        },
        |url| url
    );

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await
        .map_or_else(
            |_| {
                println!("Could not connect to database");
                std::process::exit(1);
            },
            |pool| pool
        );
    let client = Client::new(); // Reqwest client for making external API calls
    println!("running v2 :)");

    rocket
        ::build()
        .mount(
            "/v2",
            routes![
                get_queue,
                add_to_queue,
                set_queue_song_played,
                clear_queue,
                set_telemetry,
                get_song,
                set_song,
                get_cover,
                set_history,
                get_history_data,
                get_twitch_name,
                motd,
                motd_all,
                get_canvas
            ]
        )
        .manage(pool)
        .manage(client)
        .attach(Cors)
        .launch().await?;

    Ok(())
}
