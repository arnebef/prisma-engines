use crate::common::*;
use psl::render_datamodel_and_config_to_string;

#[test]
fn shadow_database_url_round_trips() {
    let schema_str = indoc!(
        r#"
        datasource myds {
          provider          = "postgresql"
          url               = "postgres://"
          shadowDatabaseUrl = env("EMPTY_SHADOW_DB URL_0129")
        }

        model Cat {
          id   Int    @id
          name String
        }
        "#
    );

    let schema = psl::parse_schema(schema_str).unwrap();
    let rendered = render_datamodel_and_config_to_string(&psl::lift(&schema), &schema.configuration);

    assert_eq!(schema_str, rendered);
}
