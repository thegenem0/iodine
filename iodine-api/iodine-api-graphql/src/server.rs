use std::{net::SocketAddr, sync::Arc};

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Extension, Router,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use iodine_common::{coordinator::CoordinatorCommand, error::Error, state::DatabaseTrait};
use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};
use tower_http::cors::Any;

use crate::gql_ops::coordinator::{MutationRoot, QueryRoot};

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub struct ContextData {
    pub db: Arc<dyn DatabaseTrait>,
    pub coordinator_cmd_tx: mpsc::Sender<CoordinatorCommand>,
}

pub struct GraphQLServer {
    db: Arc<dyn DatabaseTrait>,
    coordinator_cmd_tx: mpsc::Sender<CoordinatorCommand>,
    listen_addr: SocketAddr,
}

impl GraphQLServer {
    pub fn new(
        db: Arc<dyn DatabaseTrait>,
        coordinator_cmd_tx: mpsc::Sender<CoordinatorCommand>,
        listen_addr: String,
    ) -> Result<Self, Error> {
        let listen_addr: SocketAddr = listen_addr.parse().map_err(|e| {
            Error::Internal(format!(
                "Failed to parse listen address for GraphQL server: {}",
                e
            ))
        })?;

        Ok(Self {
            db,
            coordinator_cmd_tx,
            listen_addr,
        })
    }

    pub async fn serve(&self) -> Result<JoinHandle<()>, Error> {
        let ctx_data = ContextData {
            db: Arc::clone(&self.db),
            coordinator_cmd_tx: self.coordinator_cmd_tx.clone(),
        };

        let listen_addr = self.listen_addr;

        let handle = tokio::spawn(async move {
            let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
                .data(ctx_data)
                .finish();

            let cors = tower_http::cors::CorsLayer::new()
                .allow_methods(Any)
                .allow_origin(Any);

            let app = Router::new()
                .route("/", post(Self::graphql_handler))
                .route("/playground", get(Self::graphiql_playground))
                .layer(Extension(schema))
                .layer(cors);

            let listener = match TcpListener::bind(listen_addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    eprintln!("Failed to bind GraphQL listener: {}", e);
                    return;
                }
            };

            println!("GraphiQL playground available at http://{}", listen_addr);

            if let Err(e) = axum::serve(listener, app.into_make_service()).await {
                eprintln!("GraphQL server encountered an error: {}", e);
            } else {
                println!("GraphQL server stopped gracefully.");
            }
        });

        Ok(handle)
    }

    async fn graphql_handler(
        schema: axum::Extension<AppSchema>,
        req: GraphQLRequest,
    ) -> GraphQLResponse {
        schema.execute(req.into_inner()).await.into()
    }

    async fn graphiql_playground() -> impl IntoResponse {
        Html(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DataRouter GraphiQL</title>
    <style>
        body { height: 100vh; margin: 0; overflow: hidden; }
        #graphiql { height: 100%; }
    </style>
    <script crossorigin src="https://unpkg.com/react@18/umd/react.development.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
    <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
</head>
<body>
    <div id="graphiql">Loading...</div>

    <script src="https://unpkg.com/graphiql/graphiql.min.js"></script>

    <script>
        // Create the fetcher function
        function graphQLFetcher(graphQLParams) {
            return fetch(
                '/', // Endpoint is the root path
                {
                    method: 'post',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(graphQLParams),
                },
            ).then(response => response.json());
        }

        // Render GraphiQL component
        ReactDOM.render(
            React.createElement(GraphiQL, { fetcher: graphQLFetcher }),
            document.getElementById('graphiql'),
        );
    </script>
</body>
</html>
        "#,
        )
    }
}
