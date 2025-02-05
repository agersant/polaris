use utoipa::openapi::{
	security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
	tag::TagBuilder,
	ComponentsBuilder, ContactBuilder, InfoBuilder, License, OpenApi, OpenApiBuilder,
};

pub fn open_api() -> OpenApi {
	let auth_token_description = "Authentication token acquired from the `/auth` endpoint";

	OpenApiBuilder::new()
		.info(
			InfoBuilder::new()
				.title(env!("CARGO_PKG_NAME"))
				.version(env!("CARGO_PKG_VERSION"))
				.license(Some(License::new("MIT")))
				.contact(Some(
					ContactBuilder::new().name(Some("Antoine Gersant")).build(),
				))
				.build(),
		)
		.tags(Some([
            TagBuilder::new()
			.name("Collection")
			.description(Some("These endpoints provide information about the available songs, albums, artists and genres."))
			.build(),
            TagBuilder::new()
			.name("Media")
			.description(Some("These endpoints serve song audio and album covers."))
			.build(),
            TagBuilder::new()
			.name("User Management")
			.description(Some("These endpoints can be used to manage or sign in users of this Polaris server."))
			.build(),
            TagBuilder::new()
			.name("File Browser")
			.description(Some("These endpoints allow the music collection to be browsed according to its file hierarchy."))
			.build(),
            TagBuilder::new()
			.name("Configuration")
			.description(Some("These endpoints allow administrators to manage the server's configuration.\n\nChanges are immediately saved in the Polaris configuration file."))
			.build(),
            TagBuilder::new()
			.name("Playlists")
			.description(Some("These endpoints allow users to create, retrieve, update or delete playlists."))
			.build(),
        ]))
		.components(Some(
			ComponentsBuilder::new()
				.security_scheme(
					"auth_header",
					SecurityScheme::Http(
						HttpBuilder::new()
							.scheme(HttpAuthScheme::Bearer)
							.description(Some(auth_token_description))
							.build(),
					),
				)
				.security_scheme(
					"auth_query_param",
					SecurityScheme::ApiKey(ApiKey::Query(ApiKeyValue::with_description(
						"auth_token",
						auth_token_description,
					))),
				)
				.build(),
		))
		.build()
}
