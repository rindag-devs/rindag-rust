use std::{str::FromStr, sync::Arc, time};

use crate::{builtin, file, sandbox, test, validator, workflow};

#[tokio::test]
async fn test_generate_a_plus_b() {
  test::init();

  let gen_code = "
  #include \"testlib.h\"
  #include <iostream>
  signed main(int argc,char**argv){
    registerGen(argc,argv,1);
    int a=opt<int>(\"a\");
    int b=opt<int>(\"b\");
    std::cout<<a<<' '<<b<<'\\n';
  }
  ";

  let val_code = "
  #include \"testlib.h\"
  signed main(int argc,char**argv){
    registerValidation(argc,argv);
    inf.readInt(1,100,\"a\");
    inf.readSpace();
    inf.readInt(1,100,\"b\");
    inf.readEoln();
    inf.readEof();
  }
  ";

  let std_code = "
  #include <iostream>
  signed main(){
    int a,b;
    std::cin>>a>>b;
    std::cout<<a+b<<std::endl;
  }
  ";

  let w = workflow::Workflow {
    copy_in: [
      (
        "generator.cpp".to_string(),
        gen_code.as_bytes().to_vec().into(),
      ),
      ("std.cpp".to_string(), std_code.as_bytes().to_vec().into()),
      (
        "validator.cpp".to_string(),
        val_code.as_bytes().to_vec().into(),
      ),
      (
        "testlib.h".to_string(),
        builtin::File::from_str("testlib:testlib.h").unwrap().into(),
      ),
    ]
    .into(),
    tasks: vec![
      Box::new(workflow::JudgeBatchCmd {
        lang: "cpp".to_string(),
        args: vec![],
        exec: "std".to_string(),
        inf: "1.in".to_string(),
        copy_in: [].into(),
        copy_out: "1.ans".to_string(),
        time_limit: time::Duration::from_secs(1),
        memory_limit: 64 * 1024 * 1024,
      }),
      Box::new(workflow::GenerateCmd {
        lang: "cpp".to_string(),
        args: ["--test", "main", "--group", "1", "-a", "1", "-b", "100"]
          .iter()
          .map(|&s| s.into())
          .collect(),
        exec: "generator".to_string(),
        copy_in: [].into(),
        generated: "1.in".to_string(),
      }),
      Box::new(workflow::ValidateCmd {
        lang: "cpp".to_string(),
        args: vec![],
        exec: "validator".to_string(),
        inf: "1.in".to_string(),
        copy_in: [].into(),
        report: "1.log".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "cpp".to_string(),
        args: vec![],
        code: "generator.cpp".to_string(),
        copy_in: [("testlib.h".to_string(), "testlib.h".to_string())].into(),
        exec: "generator".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "cpp".to_string(),
        args: vec![],
        code: "std.cpp".to_string(),
        copy_in: [].into(),
        exec: "std".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "cpp".to_string(),
        args: vec![],
        code: "validator.cpp".to_string(),
        copy_in: [("testlib.h".to_string(), "testlib.h".to_string())].into(),
        exec: "validator".to_string(),
      }),
    ],
    copy_out: ["1.in".to_string(), "1.ans".to_string(), "1.log".to_string()].into(),
  };

  let sandbox = Arc::new(sandbox::Client::from_global_config().await);
  let res = w.exec(sandbox.clone()).await.unwrap();

  assert_eq!(
    sandbox
      .file_get(res["1.in"].to_string())
      .await
      .unwrap()
      .content,
    "1 100\n".as_bytes().to_vec()
  );
  assert_eq!(
    sandbox
      .file_get(res["1.ans"].to_string())
      .await
      .unwrap()
      .content,
    "101\n".as_bytes().to_vec()
  );
  let val_log: validator::Overview = bson::from_slice(
    &sandbox
      .file_get(res["1.log"].to_string())
      .await
      .unwrap()
      .content,
  )
  .unwrap();
  assert_eq!(
    val_log,
    validator::Overview {
      variables: [
        (
          "a".to_string(),
          validator::VariableBounds {
            hit_min: true,
            hit_max: false
          }
        ),
        (
          "b".to_string(),
          validator::VariableBounds {
            hit_min: false,
            hit_max: true
          }
        ),
      ]
      .into(),
      features: [].into()
    }
  )
}

#[tokio::test]
async fn test_duplicate_file() {
  test::init();

  let w = workflow::Workflow {
    copy_in: [(
      "a.c".to_string(),
      file::File::Memory("a".as_bytes().to_vec()),
    )]
    .into(),
    tasks: vec![
      Box::new(workflow::CompileCmd {
        lang: "c".to_string(),
        args: vec![],
        code: "a.c".to_string(),
        copy_in: [].into(),
        exec: "b.c".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "c".to_string(),
        args: vec![],
        code: "b.c".to_string(),
        copy_in: [].into(),
        exec: "c.c".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "c".to_string(),
        args: vec![],
        code: "c.c".to_string(),
        copy_in: [].into(),
        exec: "b.c".to_string(),
      }),
    ],
    copy_out: [].into(),
  };

  let sandbox = Arc::new(sandbox::Client::from_global_config().await);
  let err = w.exec(sandbox.clone()).await.unwrap_err();
  if let workflow::Error::Parse(workflow::ParseError::DuplicateFile(err)) = err {
    assert_eq!(
      err,
      workflow::DuplicateFileError::Prev {
        index1: 0,
        index2: 2,
        name: "b.c".to_string()
      }
    )
  } else {
    assert!(false);
  }
}
