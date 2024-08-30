pub fn assert_aws_env_is_not_set() {
    assert!(
        std::env::var("AWS_SESSION_TOKEN").is_err(),
        "AWS_SESSION_TOKEN is set. Use infratk in a new shell"
    );
}
