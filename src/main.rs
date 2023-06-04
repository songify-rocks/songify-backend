use std::env;

use rocket::{
    get, post, routes,
    serde::{json::Json, Deserialize, Serialize},
    State, futures::future::ok, http::Status, response::status::Unauthorized,
};
use sqlx::{mysql::MySqlPoolOptions, FromRow, MySql, Pool};

#[derive(FromRow, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueSong {
    Queueid: Option<i32>,
    Uuid: String,
    Trackid: String,
    Artist: String,
    Title: String,
    Length: String,
    Requester: String,
    Played: i32,
    Albumcover: Option<String>,
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

impl QueueSong {
    pub async fn get_queue(id: String, pool: &Pool<MySql>) -> Result<Vec<QueueSong>, sqlx::Error> {
        println!("Getting queue for uuid: {id}");
        let queue = sqlx::query_as::<MySql, QueueSong>(
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
}

impl Usage {
    pub async fn get_access_key(
        id: String,
        pool: &Pool<MySql>,
    ) -> Result<Option<String>, sqlx::Error> {
        let usage: Usage =
            sqlx::query_as::<MySql, Usage>("SELECT * FROM songify_usage WHERE UUID = ?")
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
}

async fn verify_access_key(uuid: &str, api_key: &str, pool: &State<Pool<MySql>>) -> Result<(), Unauthorized<()>> {
    let access_key = Usage::get_access_key(uuid.to_string(), pool).await.unwrap();
    match access_key {
        Some(key) => {
            if key != api_key {
                let status = rocket::response::status::Unauthorized::<()>(None);
                return Err(status);
            }
        }
        None => {
            Usage::set_access_key(uuid.to_string(), api_key.to_string(), pool).await.unwrap();
        }
    }

    Ok(())
}

#[get("/queue.php?<uuid>")]
async fn get_queue(pool: &State<Pool<MySql>>, uuid: &str) -> Json<Vec<QueueSong>> {
    let queue = QueueSong::get_queue(uuid.to_string(), &pool).await.unwrap();
    Json(queue)
}

#[post("/queue.php?<uuid>&<api_key>", format = "json", data = "<song>")]
async fn add_to_queue(
    pool: &State<Pool<MySql>>,
    uuid: &str,
    api_key: &str,
    song: Json<QueueSong>,
) -> Result<Json<QueueSong>, Unauthorized<()>> {
    verify_access_key(uuid, api_key, pool).await?;

    let queue_song = QueueSong::add_to_queue(uuid.to_string(), song.into_inner(), pool)
        .await
        .unwrap();

    Ok(Json(queue_song))
}
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let database_url = env::var("DATABASE_URL").unwrap_or(
        "mysql://songify:e3c5b05667f27e1db18dd8608faf0212@root.cyklan.de:3306/songify".to_string(),
    );
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    rocket::build()
        .mount("/v2", routes![get_queue, add_to_queue])
        .manage(pool)
        .launch()
        .await?;

    Ok(())
}
