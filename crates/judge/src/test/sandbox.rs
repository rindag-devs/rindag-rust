use std::sync::Arc;

use crate::sandbox;

/// A test for sandbox compiling and running a C code with gcc.
#[test]
fn test_hello_world() {
  super::async_test(async {
    let compile_res = sandbox::Request::Run(sandbox::Cmd {
      args: vec!["/usr/bin/gcc".to_string(), "a.c".to_string()],
      copy_in: [(
        "a.c".to_string(),
        Arc::new(
          sandbox::FileHandle::upload(
            "#include<stdio.h>\nint main(){puts(\"hello, world!\\n你好, 世界!\");}".as_bytes(),
          )
          .await,
        ),
      )]
      .into(),
      copy_out: vec!["a.out".to_string()],
      ..Default::default()
    })
    .exec()
    .await[0]
      .clone();

    assert_eq!(compile_res.result.status, sandbox::Status::Accepted);

    let exec_file = compile_res.files["a.out"].clone();

    let run_res = sandbox::Request::Run(sandbox::Cmd {
      args: vec!["a.out".to_string()],
      copy_in: [("a.out".to_string(), exec_file.clone())].into(),
      copy_out: vec!["stdout".to_string()],
      ..Default::default()
    })
    .exec()
    .await[0]
      .clone();

    assert_eq!(run_res.result.status, sandbox::Status::Accepted);
    assert_eq!(
      run_res.files["stdout"].context().await.unwrap(),
      "hello, world!\n你好, 世界!\n".as_bytes().to_vec()
    );
  });
}
