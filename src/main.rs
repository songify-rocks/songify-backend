use std::env;

use rocket::{
    get, post, routes,
    serde::{json::Json, Deserialize, Serialize},
    State, patch, http::Status, fairing::{Fairing, Info},
};
use sqlx::{mysql::MySqlPoolOptions, FromRow, MySql, Pool};

#[derive(Deserialize, Serialize, FromRow)]
#[serde(crate = "rocket::serde")]
struct Song {
    uuid: String,
    song: String,
    cover_url: String
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

struct Cors;

fn get_custom_uuids(uuid: &str) -> &str {
    match uuid {
         "inzaniity" => "43efb299-2504-4365-8ac6-a301f0d7c7aa",
         "thejaydizzle" => "5d07c1d6-6dcc-4185-a6bd-284fe0480b79",
         "sluckz" => "f6d9a390-7d48-4da6-a177-c378a7a33c1e",
         "vigilsc" => "2580091a-aec1-44be-afbd-274523c1b3d2",
         "itsbustre" => "c90b6e0e-6706-4036-bf25-327b2d981082",
         "rocketstarrl" => "de8a9f85-2919-474c-9845-6534ec54dc7f",
         "preheet" => "4aa39d0a-1bf6-4705-bfb5-512dd8afc1e2",
         "highitsky" => "630e6596-a833-42d9-a905-7a5bf1a75d0e",
        _ => uuid
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

    async fn on_response<'r>(&self, _request: &'r rocket::Request<'_>, response: &mut rocket::Response<'r>) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS, PATCH, DELETE"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "Content-Type"));
    }
}

impl History {
    pub async fn set_history(history: Self, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO songify_history (uuid, song, tst) VALUES (?, ?, ?)")
            .bind(history.uuid)
            .bind(history.song)
            .bind(history.tst)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_history(id: String, pool: &Pool<MySql>) -> Result<Vec<Self>, sqlx::Error> {
        let uuid = get_custom_uuids(&id);
        let history = sqlx::query_as::<MySql, Self>("SELECT * FROM songify_history WHERE uuid = ? ORDER BY tst DESC")
            .bind(uuid)
            .fetch_all(pool)
            .await?;

        Ok(history)
    }
}

impl Song {
    pub async fn set_song(song: Self, pool: &Pool<MySql>) -> sqlx::Result<()> {
        sqlx::query("REPLACE INTO song_data (UUID, song, cover_url) VALUES (?, ?, ?)")
            .bind(song.uuid)
            .bind(song.song)
            .bind(song.cover_url)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_song(id: String, pool: &Pool<MySql>) -> Result<Self, sqlx::Error> {
        let song = sqlx::query_as::<MySql, Self>("SELECT * FROM song_data WHERE uuid = ?")
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

#[get("/getsong?<uuid>")]
async fn get_song(pool: &State<Pool<MySql>>, uuid: &str) -> Result<String, Status> {
    Song::get_song(uuid.to_string(), pool).await.map_or(Err(Status::InternalServerError), |song| Ok(song.song))
}

#[get("/getcover?<uuid>")]
async fn get_cover(pool: &State<Pool<MySql>>, uuid: &str) -> Result<String, Status> {
    Song::get_song(uuid.to_string(), pool).await.map_or(Err(Status::InternalServerError), |song| Ok(song.cover_url))
}

#[get("/queue?<uuid>")]
async fn get_queue(pool: &State<Pool<MySql>>, uuid: &str) -> Result<Json<Vec<QueueSong>>, Status> {
    QueueSong::get_queue(uuid.to_string(), pool).await.map_or(Err(Status::InternalServerError), |queue| Ok(Json(queue)))
}

#[post("/queue?<api_key>", format = "json", data = "<song>")]
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

#[patch("/queue?<api_key>", format = "json", data = "<song>")]
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

#[post("/queue_delete?<api_key>", format = "json", data = "<queue>")]
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

#[post("/telemetry", format = "json", data = "<telemetry>")]
async fn set_telemetry(pool: &State<Pool<MySql>>, telemetry: Json<Telemetry>) -> Result<(), Status> {
    let data = telemetry.into_inner();
    verify_access_key(&data.uuid, &data.key, pool).await?;
    Usage::set_telemetry(data, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[post("/song?<api_key>", format = "json", data = "<song>")]
async fn set_song(pool: &State<Pool<MySql>>, api_key: String, song: Json<SongPayload>) -> Result<(), Status> {
    let data = song.into_inner();

    let cover = data.cover.map_or_else(String::new, |cover| cover);

    let song: Song = Song {
        uuid: data.uuid,
        song: data.song,
        cover_url: cover,
    };

    verify_access_key(&song.uuid, &data.key, pool).await?;
    Song::set_song(song, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[post("/history?<api_key>", format = "json", data = "<payload>")]
async fn set_history(pool: &State<Pool<MySql>>, api_key: String, payload: Json<HistoryPayload>) -> Result<(), Status> {
    let payload = payload.into_inner();
    verify_access_key(&payload.id, &api_key, pool).await?;

    let history = History {
        uuid: payload.id,
        song: payload.song,
        tst: payload.tst.to_string(),
    };

    History::set_history(history, pool).await.map_or(Err(Status::InternalServerError), |_| Ok(()))
}

#[get("/history_data?<id>")]
async fn get_history_data(pool: &State<Pool<MySql>>, id: String) -> Result<Json<Vec<History>>, Status> {
    History::get_history(id, pool).await.map_or(Err(Status::InternalServerError), |history| Ok(Json(history)))
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
        .mount("/v2", routes![
            get_queue, 
            add_to_queue, 
            set_queue_song_played, 
            clear_queue, 
            set_telemetry, 
            get_song, 
            set_song, 
            get_cover, 
            set_history, 
            get_history_data
        ])
        .manage(pool)
        .attach(Cors)
        .launch()
        .await?;

    Ok(())
}
