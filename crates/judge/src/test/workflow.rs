use std::sync::Arc;

use crate::{sandbox, test, workflow};

#[tokio::test]
async fn test_generate_a_plus_b() {
  test::init();

  let gen_code = "
  #include \"testlib.h\"
  #include <iostream>
  signed main(int argc,char**argv){
    registerGen(argc,argv,1);
    int n=opt<int>(\"n\");
    std::cout<<rnd.next(1,n)<<' '<<rnd.next(1,n)<<'\\n';
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
        workflow::File::Memory(gen_code.as_bytes().to_vec()),
      ),
      (
        "std.cpp".to_string(),
        workflow::File::Memory(std_code.as_bytes().to_vec()),
      ),
      (
        "validator.cpp".to_string(),
        workflow::File::Memory(val_code.as_bytes().to_vec()),
      ),
      (
        "testlib.h".to_string(),
        workflow::File::Builtin("testlib:testlib.h".parse().unwrap()),
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
      }),
      Box::new(workflow::GenerateCmd {
        lang: "cpp".to_string(),
        args: ["--test", "main", "--group", "1", "-n", "100", "-m", "100"]
          .iter()
          .map(|&s| s.into())
          .collect(),
        exec: "generator".to_string(),
        copy_in: [].into(),
        copy_out: "1.in".to_string(),
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
        copy_out: "generator".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "cpp".to_string(),
        args: vec![],
        code: "std.cpp".to_string(),
        copy_in: [].into(),
        copy_out: "std".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "cpp".to_string(),
        args: vec![],
        code: "validator.cpp".to_string(),
        copy_in: [("testlib.h".to_string(), "testlib.h".to_string())].into(),
        copy_out: "validator".to_string(),
      }),
    ],
    copy_out: ["1.ans".to_string(), "1.log".to_string()].into(),
  };

  let sandbox = Arc::new(sandbox::Client::from_global_config().await);
  assert!(sandbox.exec_workflow(Arc::new(w)).await.is_ok());
}

#[tokio::test]
async fn test_duplicate_file() {
  test::init();

  let w = workflow::Workflow {
    copy_in: [(
      "a.c".to_string(),
      workflow::File::Memory("a".as_bytes().to_vec()),
    )]
    .into(),
    tasks: vec![
      Box::new(workflow::CompileCmd {
        lang: "c".to_string(),
        args: vec![],
        code: "a.c".to_string(),
        copy_in: [].into(),
        copy_out: "b.c".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "c".to_string(),
        args: vec![],
        code: "b.c".to_string(),
        copy_in: [].into(),
        copy_out: "c.c".to_string(),
      }),
      Box::new(workflow::CompileCmd {
        lang: "c".to_string(),
        args: vec![],
        code: "c.c".to_string(),
        copy_in: [].into(),
        copy_out: "b.c".to_string(),
      }),
    ],
    copy_out: [].into(),
  };

  let sandbox = Arc::new(sandbox::Client::from_global_config().await);
  let err = sandbox.exec_workflow(Arc::new(w)).await.unwrap_err();
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
