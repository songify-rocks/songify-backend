use std::env;

use rocket::{
    get, post, routes,
    serde::{json::Json, Deserialize, Serialize},
    State, patch, delete, http::Status, fairing::{Fairing, Info},
};
use sqlx::{mysql::MySqlPoolOptions, FromRow, MySql, Pool};

#[derive(Deserialize, Serialize, FromRow)]
#[serde(crate = "rocket::serde")]
struct Song {
    uuid: String,
    key: String,
    song: String,
    cover: String
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
    uuid: String
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct SongPayload {
    uuid: String,
    key: String,
    song: String,
    cover: Option<String>
}


#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueUpdatePayload {
    queueid: i32,
    uuid: String
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueClearPayload {
    uuid: String,
    key: String
}

#[derive(FromRow)]
struct Usage {
    UUID: String,
    tst: String,
    twitch_id: i32,
    twitch_name: String,
    vs: String,
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
    vs: String,
    playertype: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct History {
    id: String,
    song: String,
    key: String,
    tst: i32,
}

struct CORS;
#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: rocket::fairing::Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r rocket::Request<'_>, response: &mut rocket::Response<'r>) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS, PATCH, DELETE"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "Content-Type"));
    }
}

impl History {
    pub async fn set_history(history: Self, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO songify_history (UUID, song, tst) VALUES (?, ?, ?)")
            .bind(history.id)
            .bind(history.song)
            .bind(history.tst)
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl Song {
    pub async fn set_song(song: Self, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx::query("REPLACE INTO song_data (UUID, song, cover_url) VALUES (?, ?, ?)")
            .bind(song.uuid)
            .bind(song.song)
            .bind(song.cover)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_song(id: String, pool: &Pool<MySql>) -> Result<Self, sqlx::Error> {
        let song = sqlx::query_as::<MySql, Self>("SELECT * FROM song_data WHERE UUID = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;

        Ok(song)
    }
}

impl QueueSong {
    pub async fn get_queue(id: String, pool: &Pool<MySql>) -> Result<Vec<Self>, sqlx::Error> {
        let queue = sqlx::query_as::<MySql, Self>(
            "SELECT * FROM songify_queue WHERE Uuid = ? AND Played = 0;",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        Ok(queue)
    }

    pub async fn add_to_queue(id: String, song: Self, pool: &Pool<MySql>) -> sqlx::Result<Self> {
        use sqlx::Row;

        let inserted_song = sqlx::query( 
            "INSERT INTO songify_queue (Queueid, Uuid, Trackid, Artist, Title, Length, Requester, Played, Albumcover) VALUES (NULL, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *")
            .bind(id)
            .bind(&song.Trackid)
            .bind(&song.Artist)
            .bind(&song.Title)
            .bind(&song.Length)
            .bind(&song.Requester)
            .bind(0)
            .bind(&song.Albumcover)
            .fetch_one(pool)
            .await?;

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

    pub async fn remove_from_queue(uuid: String, queueid: i32, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx::query("UPDATE songify_queue SET Played = 1 WHERE Uuid = ? AND Queueid = ?")
            .bind(uuid)
            .bind(queueid)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn clear_queue(uuid: String, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx::query("UPDATE songify_queue SET Played = 1 WHERE Uuid = ?")
            .bind(uuid)
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl Usage {
    pub async fn get_access_key(
        id: String,
        pool: &Pool<MySql>,
    ) -> Result<Option<String>, sqlx::Error> {
        let usage =
            sqlx::query_as::<MySql, Self>("SELECT * FROM songify_usage WHERE UUID = ?")
                .bind(id)
                .fetch_one(pool)
                .await?;

        Ok(usage.access_key)
    }

    pub async fn set_access_key(
        id: String,
        access_key: String,
        pool: &Pool<MySql>,
    ) -> sqlx::Result<()> {
        sqlx::query("UPDATE songify_usage SET access_key = ? WHERE UUID = ?")
            .bind(access_key)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn set_telemetry(telemetry: Telemetry, pool: &Pool<MySql>,) -> sqlx::Result<()> {
        sqlx::query("REPLACE INTO songify_usage (UUID, tst, twitch_id, twitch_name, vs, playertype, access_key) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(telemetry.uuid)
            .bind(telemetry.tst)
            .bind(telemetry.twitch_id)
            .bind(telemetry.twitch_name)
            .bind(telemetry.vs)
            .bind(telemetry.playertype)
            .bind(telemetry.key)
            .execute(pool)
            .await?;
        Ok(())
    }
}

async fn verify_access_key(uuid: &str, api_key: &str, pool: &State<Pool<MySql>>) -> Result<(), Status> {
    let access_key = Usage::get_access_key(uuid.to_string(), pool).await.map_err(|_| Status::InternalServerError)?;

    match access_key {
        Some(key) => {
            if key != api_key {
                return Err(Status::Unauthorized);
            }
        }
        None => {
            Usage::set_access_key(uuid.to_string(), api_key.to_string(), pool).await.map_err(|_| Status::InternalServerError)?;
        }
    }

    Ok(())
}

#[get("/getsong.php?<uuid>")]
async fn get_song(pool: &State<Pool<MySql>>, uuid: &str) -> Result<String, Status> {
    Song::get_song(uuid.to_string(), pool).await.map_or(Err(Status::InternalServerError), |song| Ok(song.song))
}
#[get("/getcover.php?<uuid>")]
async fn get_cover(pool: &State<Pool<MySql>>, uuid: &str) -> Result<String, Status> {
    Song::get_song(uuid.to_string(), pool).await.map_or(Err(Status::InternalServerError), |song| Ok(song.cover))
}

#[get("/queue.php?<uuid>")]
async fn get_queue(pool: &State<Pool<MySql>>, uuid: &str) -> Result<Json<Vec<QueueSong>>, Status> {
    QueueSong::get_queue(uuid.to_string(), pool).await.map_or(Err(Status::InternalServerError), |queue| Ok(Json(queue)))
}

#[post("/queue.php?<api_key>", format = "json", data = "<song>")]
async fn add_to_queue(
    pool: &State<Pool<MySql>>,
    api_key: &str,
    song: Json<QueuePostPayload>,
) -> Result<Json<QueueSong>, Status> {
    let song = song.into_inner();
    verify_access_key(&song.uuid, api_key, pool).await?;

    (QueueSong::add_to_queue(song.uuid, song.queueItem, pool)
        .await).map_or(Err(Status::InternalServerError), |song| Ok(Json(song)))
}

#[patch("/queue.php?<api_key>", format = "json", data = "<song>")]
async fn set_queue_song_played(pool: &State<Pool<MySql>>, api_key: &str, song: Json<QueueUpdatePayload>) -> Result<(), Status> {
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

#[post("/queue_delete.php?<api_key>", format = "json", data = "<queue>")]
async fn clear_queue(pool: &State<Pool<MySql>>, api_key: &str, queue: Json<QueueClearPayload>) -> Result<(), Status> {
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

#[post("/telemetry.php", format = "json", data = "<telemetry>")]
async fn set_telemetry(pool: &State<Pool<MySql>>, telemetry: Json<Telemetry>) -> Result<(), Status> {
    let data = telemetry.into_inner();
    verify_access_key(&data.uuid, &data.key, pool).await?;
    Usage::set_telemetry(data, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[post("/song.php?<api_key>", format = "json", data = "<song>")]
async fn set_song(pool: &State<Pool<MySql>>, api_key: String, song: Json<SongPayload>) -> Result<(), Status> {
    let data = song.into_inner();

    let cover = data.cover.map_or_else(String::new, |cover| cover);

    let song: Song = Song {
        uuid: data.uuid,
        song: data.song,
        cover,
        key: api_key,
    };

    verify_access_key(&song.uuid, &data.key, pool).await?;
    Song::set_song(song, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[post("/history.php?<api_key>", format = "json", data = "<history>")]
async fn set_history(pool: &State<Pool<MySql>>, api_key: String, history: Json<History>) -> Result<(), Status> {
    let history = history.into_inner();
    verify_access_key(&history.id, &api_key, pool).await?;

    History::set_history(history, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let database_url = env::var("DATABASE_URL")
    .map_or_else(|_| {
        println!("No database url found");
        std::process::exit(1);
    }, |url| url);
    

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_or_else(|_| {
            println!("Could not connect to database");
            std::process::exit(1);
        }, |pool| pool);



    rocket::build()
        .mount("/v2", routes![get_queue, add_to_queue, set_queue_song_played, clear_queue, set_telemetry, get_song, set_song, get_cover, set_history])
        .manage(pool)
        .attach(CORS)
        .launch()
        .await?;

    Ok(())
}
