use crate::tensorflow_bindings::*;

use std::fs::{File, create_dir, read_dir};
use std::io::prelude::*;
use std::ptr::{null, null_mut};




fn Okay(status: *mut TF_Status)->Result<(),()> {unsafe{
  if (TF_GetCode(status) != TF_Code::TF_OK) {
        println!("ERROR: {:?}\n", std::ffi::CStr::from_ptr(TF_Message(status)));
        return Err(());
  }
  return Ok(());
}}


pub struct TGBasicModel{
    graph:   *mut TF_Graph,
    session: *mut TF_Session,
    status:  *mut TF_Status,

    input:   TF_Output,
    output:  TF_Output,
}

impl TGBasicModel{
    pub fn new()->TGBasicModel{unsafe{
        TGBasicModel{
            graph:   null_mut(),
            session: null_mut(),
            status:  null_mut(),

            input: TF_Output::new(),
            output: TF_Output::new(),
        }
    }}
    pub fn init(&mut self)->Result<(), String>{unsafe{
        self.status = TF_NewStatus();
        self.graph = TF_NewGraph();
        {
            // Create the session.
            //TODO
            //What is a session
            let opts = TF_NewSessionOptions();
            self.session = TF_NewSession(self.graph, opts, self.status);
            TF_DeleteSessionOptions(opts);
            if Okay(self.status).is_err() {return Err("Status fail.".to_string());}
        }
        Ok(())
    }}
    pub fn load_graph_from_file(&mut self, filename: &str, io: Option<[&str; 2]>)->Result<(), String>{unsafe{
        let init_result = self.init();
        match init_result {
            Err(e)=>return Err(e),
            _=>{},
        }

        let mut g = self.graph;

        {
            // Import the graph.
            let mut graph_def  = { //Load file
                let mut file = File::open(filename).expect(&format!("File not there or could not be opened. {}", filename));

                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).expect("File could not be read.");
                let mut ret = TF_NewBufferFromString(&buffer[0] as *const u8 as *const ::std::os::raw::c_void, buffer.len());
                if ret == null_mut(){ println!("Failed to read buffer");}
                ret
            };
            //let mut graph_def = ReadFile(filename);
            if graph_def == null_mut(){ return Err("Graph dead!".to_string());}
            let mut opts = TF_NewImportGraphDefOptions();
            TF_GraphImportGraphDef(g, graph_def, opts, self.status);

            TF_DeleteImportGraphDefOptions(opts);
            TF_DeleteBuffer(graph_def);

            if Okay(self.status).is_err() { return Err("Status fail!".to_string()); }
        }

        match io{
            Some(s) =>{
                self.input.oper = TF_GraphOperationByName(g, s[0].as_ptr());
                self.input.index = 0;
                self.output.oper = TF_GraphOperationByName(g, s[1].as_ptr());
                self.output.index = 0;
            },
            None => {
                let mut cursor = 0;
                let mut pos = 0usize;
                let mut oper = TF_GraphNextOperation(g, &mut pos as *mut usize);
                let mut op_name = std::ffi::CStr::from_ptr("\0".as_ptr() as *mut _);
                while (oper != null_mut()) {
                    op_name = std::ffi::CStr::from_ptr(TF_OperationName(oper));
                    if cursor == 0 {
                        self.input.oper = TF_GraphOperationByName(g, op_name.as_ptr() as *const _);
                        self.input.index = 0;
                    }
                    oper = TF_GraphNextOperation(g, &mut pos as *mut usize);
                    cursor += 1;
                }
                self.output.oper = TF_GraphOperationByName(g, op_name.as_ptr() as *const _);
                self.output.index = 0;
            }
        }

        if self.input.oper == null_mut() { return Err("input DEAD".to_string());}
        if self.output.oper == null_mut() { return Err("output DEAD".to_string());}

        return Ok(());
    }}

    //TODO should prob combine these two functions
    pub fn get_input_dimensions(&self)->Vec<i64>{unsafe{
        let mut rt = vec![];
        let n_dims = TF_GraphGetTensorNumDims(self.graph, self.input, self.status);
        if Okay(self.status).is_err() { return rt;}
        for i in 0..n_dims {
            rt.push(0);
        }
        TF_GraphGetTensorShape(self.graph, self.input, rt.as_mut_ptr(), n_dims, self.status);
        return rt;
    }}
    pub fn get_output_dimensions(&self)->Vec<i64>{unsafe{
        let mut rt = vec![];
        let n_dims = TF_GraphGetTensorNumDims(self.graph, self.output, self.status);
        if Okay(self.status).is_err() { return rt;}
        for i in 0..n_dims {
            rt.push(0);
        }
        TF_GraphGetTensorShape(self.graph, self.output, rt.as_mut_ptr(), n_dims, self.status);
        return rt;
    }}

    pub fn predict(&self, batch_input: &mut[f32], batch_size: u32)->Result<Vec<f32>,()>{unsafe{
        let mut dims = self.get_input_dimensions();
        let mut nbytes = (batch_size as usize) * 4;
        for i in 1..dims.len() as usize{
            nbytes *= dims[i] as usize;
        }
        dims[0] = batch_size as i64;
        let t = TF_AllocateTensor(TF_DataType::TF_FLOAT, dims.as_ptr(), 4, nbytes) as *mut TF_Tensor;
        std::ptr::copy_nonoverlapping(batch_input.as_mut_ptr() as *mut std::ffi::c_void, TF_TensorData(t),  nbytes);


        let inputs = [self.input];
        let input_values = [t as *const _TF_Empty_Struct];
        let outputs = [self.output];
        let mut output_values = [null_mut()];


        TF_SessionRun(self.session, null(), inputs.as_ptr(), input_values.as_ptr(), 1, outputs.as_ptr(),
                    output_values.as_mut_ptr(), 1,
                    /* No target operations to run */
                    null(), 0, null_mut(), self.status);
        TF_DeleteTensor(t);
        if Okay(self.status).is_err() { return Err(()); }

        let mut pred_nbytes = 4 * batch_size as usize;
        let out_dims = self.get_output_dimensions();
        for i in 0..out_dims.len() as usize{
            pred_nbytes *= out_dims[i] as usize;
        }
        if TF_TensorByteSize(output_values[0]) != pred_nbytes {
            println!("{}", output_values[0] == null_mut());
            println!("ERROR: Expected predictions tensor to have {:?} bytes, has {:?}\n",
                    pred_nbytes, TF_TensorByteSize(output_values[0]));
            TF_DeleteTensor(output_values[0]);
            return Err(());
        }

        let mut rt = vec![0.0f32; pred_nbytes / 4];
        std::ptr::copy_nonoverlapping(TF_TensorData(output_values[0]), rt.as_mut_ptr() as *mut std::ffi::c_void, pred_nbytes);
        let data_ptr = (TF_TensorData(output_values[0]) as *mut f32);
        TF_DeleteTensor(output_values[0]);
        return Ok(rt);
    }}

}

pub mod Ops{
use crate::tensorflow_bindings::*;

use std::fs::{File, create_dir, read_dir};
use std::io::prelude::*;
use std::ptr::{null, null_mut};


    pub fn AddN(l: &[*mut TF_Operation], r: *mut TF_Operation, graph: *mut TF_Graph,
                   s: *mut TF_Status, check: bool)->*mut TF_Operation {unsafe{

        let desc = TF_NewOperation(graph, "AddN\0".as_ptr() as *const _, "test\0".as_ptr() as *const _);

        let mut add_inputs = Vec::new();
        for it in l{
            add_inputs.push(TF_Output{oper: it.clone(), index: 0});
        }
        TF_AddInputList(desc, add_inputs.as_ptr() as *mut _, add_inputs.len() as _);

        let op = TF_FinishOperation(desc, s);

        assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "add ops: {:?}", &std::ffi::CStr::from_ptr(TF_Message(s)) );
        assert_ne!(op, null_mut());

        return op;
    }}


    macro_rules! op_algebra_helper{
        //TODO
        //change name of function to something less general
        ($ops:tt)=>{  pub fn $ops(l: *mut TF_Operation, r: *mut TF_Operation, graph: *mut TF_Graph, 
                              s: *mut TF_Status, name: &str)->*mut TF_Operation {unsafe{

                          let ops_string = std::ffi::CString::new(stringify!($ops)).expect("No a proper cstring");
                          let name_string = std::ffi::CString::new(name).expect("No a proper cstring");

                          //TODO 
                          //"test" needs to be removed for something more dynamic
                          let desc = TF_NewOperation(graph, ops_string.into_raw() as *const _, name_string.into_raw() as *const _);

                          let add_inputs = [TF_Output{oper: l, index: 0}, TF_Output{ oper: r, index: 0}];
                          TF_AddInput(desc, add_inputs[0]);
                          TF_AddInput(desc, add_inputs[1]);

                          let op = TF_FinishOperation(desc, s);

                          assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "{} ops: {:?}", stringify!($ops), &std::ffi::CStr::from_ptr(TF_Message(s)) );
                          assert_ne!(op, null_mut());

                          return op;
                      }}
                   };
    }

    op_algebra_helper!(AddV2);
    op_algebra_helper!(Sub);
    op_algebra_helper!(Mul);
    op_algebra_helper!(Div);
    op_algebra_helper!(Pow);
    op_algebra_helper!(Mod);

    pub fn PlaceholderHelper(graph: *mut TF_Graph, s: *mut TF_Status, name: &str,
                           dtype: TF_DataType, dims: &[i64])->TF_Output {unsafe{

        let _name = std::ffi::CString::new(name).unwrap();
        let desc = TF_NewOperation(graph, "Placeholder\0".as_ptr() as *const _, _name.into_raw() as *const _);

        TF_SetAttrType(desc, "dtype\0".as_ptr() as *const _, dtype);
        if (dims.len() != 0) {
            TF_SetAttrShape(desc, "shape\0".as_ptr() as *const _, dims.as_ptr(), dims.len() as i32);
        }
        let op = TF_FinishOperation(desc, s);

        assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "{:?}", &std::ffi::CStr::from_ptr(TF_Message(s)));
        assert_ne!(op, null_mut());
        
        return TF_Output{ oper: op, index: 0};
    }}



    pub fn ConstHelper(t: *mut TF_Tensor, graph: *mut TF_Graph, s: *mut TF_Status,
                       name: &str)->*mut TF_Operation {unsafe{
        let _name = std::ffi::CString::new(name).expect("const helper");

        let desc = TF_NewOperation(graph, "Const\0".as_ptr() as *const _, _name.into_raw() as *mut _);
        TF_SetAttrTensor(desc, "value\0".as_ptr() as *mut _, t, s);

        assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "const helper {:?}", &std::ffi::CStr::from_ptr(TF_Message(s)) );

        TF_SetAttrType(desc, "dtype\0".as_ptr() as *mut _, TF_TensorType(t));
        let op = TF_FinishOperation(desc, s);

        assert_eq!(TF_Code::TF_OK, TF_GetCode(s), "const helper 2 {:?}",TF_Message(s));
        assert_ne!(op, null_mut());

        return op;
    }}


    //TODO
    //should not be placed here also this is too restictive to be a proper wrapper
    pub fn FloatTensor(v: f32)->*mut TF_Tensor {unsafe{
        let num_bytes = 4;
        let values = [v];
        let t = TF_NewTensor(TF_DataType::TF_FLOAT, null(), 0, values.as_ptr() as *mut _, num_bytes,
                            deallocator, null_mut());

        return t;
    }}



    unsafe extern "C" fn deallocator(data: *mut std::ffi::c_void, _u: usize, _ptr: *mut std::ffi::c_void) { unsafe{ 
          //NOTE 
          //I don't think this drops a damn thing
          //println!("trying to dealloc but we can't right now!!");
    }}
}
