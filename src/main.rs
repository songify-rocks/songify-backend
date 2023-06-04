use std::env;

use rocket::{
    get, routes,
    serde::{json::Json, Deserialize, Serialize},
    State,
};
use sqlx::{mysql::MySqlPoolOptions, FromRow, MySql, Pool};

#[derive(Debug)]
struct SongData {
    uuid: String,
    song: String,
    cover_url: String,
}
#[derive(FromRow, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct QueueSong {
    Uuid: String,
    Trackid: String,
    Artist: String,
    Title: String,
    Length: String,
    Requester: String,
    Played: bool,
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
}

#[get("/queue.php?<uuid>")]
async fn get_queue(pool: &State<Pool<MySql>>, uuid: &str) -> Json<Vec<QueueSong>> {
    let queue = QueueSong::get_queue(uuid.to_string(), &pool).await.unwrap();
    // output length of queue in console
    println!("Queue length: {}", queue.len());

    Json(queue)
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
        .mount("/v1", routes![get_queue])
        .manage(pool)
        .launch()
        .await?;

    Ok(())
}
