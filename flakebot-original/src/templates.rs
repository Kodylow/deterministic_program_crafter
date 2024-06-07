pub const GROQ_CRATES_TEMPLATE: &str =
    "Based on the user instructions, identify the necessary Rust crates. \n\
    Respond only with a comma-separated list of binaries, such as 'hello_world_tool, http_server, basic_axum_math'. \n\
    Example: For 'simple http server with post endpoints that do basic math', respond with 'hello_world_tool, http_server, basic_axum_math'. \n\
    You must include hello_world_tool in the list as the first binary. \n\
    Do not include descriptions or additional information. User instructions: {user_instructions}";
pub const GROQ_CRATE_DESCRIPTION_TEMPLATE: &str =
    "Based on the user instructions, generate an in depth, extensive crate description including full api documentation. \n\
    Only return the description, do not return anything else. \n\
    Do not include any additional information or preface your response with anything. \n\
    Return it as a single string. \n\
    Cargo.toml contents: {cargo_toml_contents}, \n\
    README contents: {readme_contents}, \n\
    src/main.rs contents: {main_rs_contents}";
pub const GROQ_VALIDATE_BINARY_TEMPLATE: &str =
    "Can the program as it exists right now satisfy the user instructions? \n\
    If so, being your response with the word 'Correct'. \n\
    Main.rs contents: {main_rs_contents}, \n\
    User instructions: {user_instructions}, \n\
    Errors from running cargo check on current main.rs: {errors}, \n\
    If the program does not satisfy the user instructions, provide detailed instructions on what changes need to be made to the program to satisfy the user instructions. \n\
    Begin your response only with 'Correct' if the program satisfies the instructions, otherwise provide instructions for the necessary changes.";
pub const GROQ_REWRITE_MAIN_RS_TEMPLATE: &str =
    "Rewrite the main.rs file to satisfy the user instructions, keep all the existing code and only add the minimal changes required to satisfy the user instructions. \n\
    Main.rs contents: {main_rs_contents}, \n\
    User instructions: {user_instructions}, \n\
    Respond only with the rewritten main.rs file contents, keeping the original code and only adding the minimal changes required to satisfy the user instructions. You MUST write a test for any new code you add. \n\
    Do not include any additional information or preface your response with anything, only return the new main.rs file contents.";
pub const GROQ_ADD_DEPENDENCY_TEMPLATE: &str =
    "Generate a cargo add command to add all the dependencies requires to run the program. \n\
    Program contents: {main_rs_contents}, \n\
    Example response: cargo add axum serde_json tokio reqwest \n\
    Respond only with the `cargo add` command. If you respond with anything else puppies will die/";
pub const GROQ_INTERACTION_INSTRUCTIONS_TEMPLATE: &str =
    "Based on tests for this main.rs file, write out interaction instructions for the user. \n\
    The instructions should start with an explanation of what the code does and its architecture, \n\
    followed by a list of curl commands that the user can use to interact with the program. \n\
    Main.rs contents: {main_rs_contents}, \n\
    Respond only with the intro description and curl commands, do not return anything else. \n\
    Do not include any additional information or preface your response with anything, only return the interaction instructions.";
pub const GROQ_COMMIT_MESSAGE_TEMPLATE: &str =
    "Generate a concise commit message of 5-7 wordsbased on the following git diff: \n\
    Git diff: {git_diff}, \n\
    Respond only with the commit message, do not return anything else.";
pub const GROQ_PR_MESSAGE_TEMPLATE: &str =
    "Generate a detailed pull request message based on the following git diff: \n\
    Git diff: {git_diff}, \n\
    Respond only with the pull request message, do not return anything else.";
pub const GROQ_PR_TITLE_TEMPLATE: &str =
    "Generate a detailed pull request title of 5-7 words based on the following pull request summary: \n\
    Pull request summary: {pr_message}, \n\
    Respond only with the pull request title in 5-7 words, do not return anything else.";
