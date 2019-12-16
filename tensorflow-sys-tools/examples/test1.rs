extern crate tensorflow_sys_tools;

use tensorflow_sys_tools::tensorflow_tools::Ops::*;
use tensorflow_sys_tools::tensorflow_bindings::*;
use std::ptr::{null, null_mut};

//TODO
// move raw api to tools
// write a logical set of exampls/test 
//  + Add, sub, mult
//  + one graph different inputs
//  + fit example



//const c_a : f32 = 4.0;
//const c_b : f32 = 34.0;

fn main(){ unsafe{

    tensorflow_init(Some("C:\\Users\\thoth\\Documents\\Rust\\screen_capturer\\tensorflow-sys-tools\\tensorflow_assets\\tensorflow.dll\0")).expect("Tensorflow lib init problem.");
    println!("ASDFASDF"); 
    let buffer = TF_GetAllOpList();
    println!("{:?}", *buffer);
    for i in 0..(*buffer).length{
        let n = *((*buffer).data as *mut u8).offset(i as isize);
        let l : char = n.into();
        print!("{}",l);
    }
    println!("");

    //algebraic_examples();
    //input_examples();
    fit_example();
}}


fn input_examples(){unsafe{
    {  
        println!("//////////// Variable input for algebriac functions in one graph ///////////");
        println!("Graph: a + x * (a - b)"); 
        let rust_a = 100.0;
        let rust_b = 10.0;
        let mut rust_x = [0.0, -12.4, 12.243];
        println!("Where a = {}; \nWhere b = {}\nwhere x = {:?}", rust_a, rust_b, rust_x);

        let mut a = FloatTensor(100.0); 
        let mut b = FloatTensor(10.0);
        
        let mut graph  = TF_NewGraph();
        let mut status = TF_NewStatus();

        let x = PlaceholderHelper(graph, status, "x", TF_DataType::TF_FLOAT, &[1]);

        let _a = ConstHelper(a, graph, status, "a");
        let _b = ConstHelper(b, graph, status, "b");

        let a_min_b = Sub(_a, _b, graph, status, "sub");
        let b_mul_  = Mul(x.oper, a_min_b, graph, status, "mul");
        let a_add_  = AddV2(_a, b_mul_, graph, status, "add");

        let opts = TF_NewSessionOptions();
        let session = TF_NewSession(graph, opts, status);

        let mut output = [TF_Output::new()];
        output[0].oper = a_add_;
        let mut output_values = [null_mut()];
        

        for r_x in rust_x.iter(){
        let initial_x = [FloatTensor(*r_x) as *const _]; 

            TF_SessionRun(session, null(), &x as *const _, initial_x.as_ptr(), 1, output.as_ptr(),
                        output_values.as_mut_ptr(), 1,
                        /* No target operations to run */
                        null(), 0, null_mut(), status);

            let _result = *(TF_TensorData(output_values[0]) as *const f32);
            println!("\n\t\texpected: {}\n\t\tresult: {}\n\t\tIs result good? {}",(rust_a + *r_x * (rust_a - rust_b)), _result, _result == (rust_a + *r_x * (rust_a - rust_b)));
        }

        TF_CloseSession(session, status);
        TF_DeleteSession(session, status);

    }
}}

fn algebraic_examples(){unsafe{

    {
        println!("/////// Addition of const scalar values ///////////");

        let rust_a = 100.0;
        let rust_b = 10.0;
       
        println!("Graph: a + b"); 
        println!("Where a = {}; \nWhere b = {}", rust_a, rust_b);


        let mut a = FloatTensor(rust_a); 
        let mut b = FloatTensor(rust_b);
        
        let mut graph  = TF_NewGraph();
        let mut status = TF_NewStatus();


        let _a = ConstHelper(a, graph, status, "a");
        let _b = ConstHelper(b, graph, status, "b");

    
        let result = AddV2(_a, _b, graph, status, "result");
    
        let opts = TF_NewSessionOptions();
        let session = TF_NewSession(graph, opts, status);

        let mut output = [TF_Output::new()];
        output[0].oper = result;
        let mut output_values = [null_mut()];


        TF_SessionRun(session, null(), null(), null(), 0, output.as_ptr(),
                    output_values.as_mut_ptr(), 1,
                    /* No target operations to run */
                    null(), 0, null_mut(), status);

        let _result = *(TF_TensorData(output_values[0]) as *const f32);
        println!("expected: 110.0\nresult: {}\nIs result good? {}", _result, _result == (rust_a + rust_b));

        TF_CloseSession(session, status);
        TF_DeleteSession(session, status);

    } 

    println!("\n\n\n");
    {
        println!("/////// Subtraction of const scalar values ///////////");
        println!("Graph: a - b"); 

        let rust_a = 100.0;
        let rust_b = 10.0;
        println!("Where a = {}; \nWhere b = {}", rust_a, rust_b);

        let mut a = FloatTensor(100.0); 
        let mut b = FloatTensor(10.0);
        
        let mut graph  = TF_NewGraph();
        let mut status = TF_NewStatus();


        let _a = ConstHelper(a, graph, status, "a");
        let _b = ConstHelper(b, graph, status, "b");

    
        let result = Sub(_a, _b, graph, status, "result");
    
        let opts = TF_NewSessionOptions();
        let session = TF_NewSession(graph, opts, status);

        let mut output = [TF_Output::new()];
        output[0].oper = result;
        let mut output_values = [null_mut()];


        TF_SessionRun(session, null(), null(), null(), 0, output.as_ptr(),
                    output_values.as_mut_ptr(), 1,
                    /* No target operations to run */
                    null(), 0, null_mut(), status);

        let _result = *(TF_TensorData(output_values[0]) as *const f32);
        println!("expected: 110.0\nresult: {}\nIs result good? {}", _result, _result == (rust_a - rust_b));

        TF_CloseSession(session, status);
        TF_DeleteSession(session, status);
    } 

    println!("\n\n\n");
    {  
        println!("//////////// Combining many algebriac functions in one graph ///////////");
        println!("Graph: a + b * (a - b)"); 
        let rust_a = 100.0;
        let rust_b = 10.0;
        println!("Where a = {}; \nWhere b = {}", rust_a, rust_b);

        let mut a = FloatTensor(100.0); 
        let mut b = FloatTensor(10.0);
        
        let mut graph  = TF_NewGraph();
        let mut status = TF_NewStatus();


        let _a = ConstHelper(a, graph, status, "a");
        let _b = ConstHelper(b, graph, status, "b");

        let a_min_b = Sub(_a, _b, graph, status, "sub");
        let b_mul_  = Mul(_b, a_min_b, graph, status, "mul");
        let a_add_  = AddV2(_a, b_mul_, graph, status, "add");

        let opts = TF_NewSessionOptions();
        let session = TF_NewSession(graph, opts, status);

        let mut output = [TF_Output::new()];
        output[0].oper = a_add_;
        let mut output_values = [null_mut()];
        
        TF_SessionRun(session, null(), null(), null(), 0, output.as_ptr(),
                    output_values.as_mut_ptr(), 1,
                    /* No target operations to run */
                    null(), 0, null_mut(), status);

        let _result = *(TF_TensorData(output_values[0]) as *const f32);
        println!("expected: ??\nresult: {}\nIs result good? {}", _result, _result == (rust_a + rust_b * (rust_a - rust_b)));

        TF_CloseSession(session, status);
        TF_DeleteSession(session, status);

    }
    println!("\n\n\n");

}}

fn fit_example(){unsafe{
    println!("//////////// Variable input for algebriac functions in one graph ///////////");
    println!("Graph: a * x + b"); 

    let mut graph  = TF_NewGraph();
    let mut status = TF_NewStatus();

    let x = PlaceholderHelper(graph, status, "x", TF_DataType::TF_FLOAT, &[1]);

    let a = VariableHelper(graph, &[1], TF_DataType::TF_FLOAT, status,  "a");
    let b = VariableHelper(graph, &[1], TF_DataType::TF_FLOAT, status,  "b");

    

    let a_x_mult = Mul(a, x.oper, graph, status, "mut");
    let result   = AddV2(b, a_x_mult, graph, status, "result");

    let mut output = [TF_Output::new()];
    output[0].oper = result;
    //let mut output_values = [null_mut()];
    
    //TF_SessionRun(session, null(), null(), null(), 0, output.as_ptr(),
    //            output_values.as_mut_ptr(), 1,
    //            /* No target operations to run */
    //            null(), 0, null_mut(), status);

    //let _result = *(TF_TensorData(output_values[0]) as *const f32);
    //println!("expected: ??\nresult: {}\nIs result good? {}", _result, _result == (rust_a + rust_b * (rust_a - rust_b)));

    //TF_CloseSession(session, status);
    //TF_DeleteSession(session, status);

}}

fn VariableHelper(graph: *mut TF_Graph, dims: &[i64],
                dtype: TF_DataType, s: *mut TF_Status, name: &str,)->*mut TF_Operation {unsafe{

    let _name = std::ffi::CString::new(name).unwrap();
    let desc = TF_NewOperation(graph, "Variable\0".as_ptr() as *const _, _name.into_raw() as *const _);

    /*TODO
      Replace dtype with Tf_OperationOutputType
    pub fn TF_OperationOutputType(oper_out: TF_Output) -> TF_DataType;
    */
    TF_SetAttrType(desc, "dtype\0".as_ptr() as *const _, dtype);
    TF_SetAttrShape(desc, "shape\0".as_ptr() as *const _, dims.as_ptr(), dims.len() as _);

    let op = TF_FinishOperation(desc, s);
    assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "{:?}", &std::ffi::CStr::from_ptr(TF_Message(s)));
    assert_ne!(op, null_mut());

    return op;
}}


fn AssignHelper(graph: *mut TF_Graph, variable_ref: *mut TF_Operation, value: *mut TF_Operation,
                dtype: TF_DataType, use_locking: u8, validate_shape: u8, s: *mut TF_Status, name: &str,)->*mut TF_Operation {unsafe{

    let _name = std::ffi::CString::new(name).unwrap();
    let desc = TF_NewOperation(graph, "Assign\0".as_ptr() as *const _, _name.into_raw() as *const _);

    TF_AddInput(desc, TF_Output{ oper: variable_ref, index: 0});
    TF_AddInput(desc, TF_Output{ oper: value, index: 0} );
    /*TODO
      Replace dtype with Tf_OperationOutputType
    pub fn TF_OperationOutputType(oper_out: TF_Output) -> TF_DataType;
    */
    TF_SetAttrType(desc, "dtype\0".as_ptr() as *const _, dtype);
    TF_SetAttrBool(desc, "validate_shape\0".as_ptr() as *const _, validate_shape);
    TF_SetAttrBool(desc, "use_locking\0".as_ptr() as *const _, use_locking);

    let op = TF_FinishOperation(desc, s);
    assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "{:?}", &std::ffi::CStr::from_ptr(TF_Message(s)));
    assert_ne!(op, null_mut());
    /*
        ref             <= takes a variable

        value           <= values what will replace variable
        output_ref      <= operation
        type            <= operation should hold the type ??
        validate_shape bool  <= i think this is optional 
        use_locking bool     <= i think this is optional
    */
    return op;
}}

fn L2LossHelper(graph: *mut TF_Graph, oper: *mut TF_Operation,
                    s: *mut TF_Status, name: &str,
                    dtype: TF_DataType)->*mut TF_Operation {unsafe{

    let _name = std::ffi::CString::new(name).unwrap();
    let desc = TF_NewOperation(graph, "L2Loss\0".as_ptr() as *const _, _name.into_raw() as *const _);

    TF_SetAttrType(desc, "dtype\0".as_ptr() as *const _, dtype);
    TF_AddInput(desc, TF_Output{ oper: oper, index: 0} );
    let op = TF_FinishOperation(desc, s);

    assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "{:?}", &std::ffi::CStr::from_ptr(TF_Message(s)));
    assert_ne!(op, null_mut());
    
    return op;
}}

//TODO
fn RandomStandardNormal(){
}

//TODO
fn Cast(){
}

//TODO
fn ApplyGradientDescent(){
}

//TODO
fn SymbolicGradient(){
}
