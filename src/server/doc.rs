use utoipa::openapi::{
	security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
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
