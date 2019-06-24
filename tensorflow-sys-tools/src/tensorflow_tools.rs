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
    pub fn load_graph_from_file(&mut self, filename: &str, io: Option<[&str; 2]>)->Result<(), &str>{unsafe{
        self.status = TF_NewStatus();
        self.graph = TF_NewGraph();
        {
            // Create the session.
            //TODO
            //What is a session
            let opts = TF_NewSessionOptions();
            self.session = TF_NewSession(self.graph, opts, self.status);
            TF_DeleteSessionOptions(opts);
            if Okay(self.status).is_err() {return Err("Status fail.");}
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
            if graph_def == null_mut(){ return Err("Graph dead!");}
            let mut opts = TF_NewImportGraphDefOptions();
            TF_GraphImportGraphDef(g, graph_def, opts, self.status);

            TF_DeleteImportGraphDefOptions(opts);
            TF_DeleteBuffer(graph_def);

            if Okay(self.status).is_err() { return Err("Status fail!"); }
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

        if self.input.oper == null_mut() { return Err("input DEAD");}
        if self.output.oper == null_mut() { return Err("output DEAD");}

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
