#[cfg(test)]
mod tests {

    use crate::common::{get_app, get_configuration};
    use actix_web::test;

    #[actix_web::test]
    async fn test_friendship_lifecycle() {
        let config = get_configuration().await;

        let app = test::init_service(get_app(config, None).await).await;

        // mock synapse 

        // test 1

        // user A request user B
        // assert not friends in db yet
        
        // user A cancel request for user B
        // assert not friends in db yet
        

        // test 2

        // user A request user B
        // assert not friends in db yet
        // assert history only has request

        // user B reject user A
        // assert not friends in db yet
        // assert history has request and reject
        
        // test 3

        // user A request user B
        // assert not friends in db yet
        // assert history only has request

        // user B accept user A
        // assert friends in db
        // assert history has request and accept


        // test 4

        // user A request user B
        // user B accept user A
        // assert friends in db
        // user B delete user A
        // assert not friends in db
        // assert history has request, accept, delete by B

        // test 5

        // user A request user B
        // user B request user A
        // assert friends in db
        // assert history has request and accept
    }
}
