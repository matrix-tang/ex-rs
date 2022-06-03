pub mod cache;


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache() {
        println!("----- test cache");
        _ = crate::db::init_db().await.unwrap();
        let mut client = crate::db::get_redis_connection().await.unwrap();
        let set_result = cache::set_ex(&mut client, "hello", &"word", 10 as usize).await;
        let get_result: String = cache::get(&mut client, "hello").await.unwrap();
        println!("{:?}, {:?}", set_result, get_result);
    }
}