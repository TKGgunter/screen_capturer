## env gpu

###############################
#Setting up imports for notebooks

from PIL import Image, ImageDraw, ImageFont
import numpy as np

import matplotlib.pyplot as plt
import scipy

import os
import os.path as osp
import argparse

import tensorflow as tf
from tensorflow import keras
from tensorflow.keras.models import Sequential, Model
from tensorflow.keras.layers import Input, Dense, Dropout, Flatten, Conv2D, MaxPooling2D
import tensorflow.keras.utils as np_utils 
import copy
###############################

def text_phantom(text, size, fontname="NotoSans-Regular",
                 fontpath="../Rust/screen_capturer/assets/", random=True,
                rand_size=(0,3) ):
    # Availability is platform dependent
    font = '../Rust/screen_capturer/assets/' + fontname

    # create a blank canvas with extra space between lines
    canvas = Image.new('L', [size, size], 255)
    # Create font
    if random == True:
        size += (np.random.randint(rand_size[0], rand_size[1]) - 1) // len(text)
    pil_font = ImageFont.truetype(font + ".ttf", size=size,
                                  encoding="unic")
    text_width, text_height = pil_font.getsize(text)


    # draw the text onto the canvas
    # TODO:
    # add some random offsets to make classifier training set more robust
    a,b = pil_font.getoffset(text)
    rand1 = -1*(a + 1)
    if b > 5:
        b = 0
    rand2 = 5 - b
    if random == True:
        rand1 += np.random.randint(0,6)
        rand2 += np.random.randint(0,2)
    draw = ImageDraw.Draw(canvas)
    offset = ((size - text_width) // 2 + rand1,
              (size - text_height) // 2 - rand2)
    white = "#000000"
    draw.text(offset, text, font=pil_font, fill=white)

    # Convert the canvas into an array with values in [0, 1]
    return (255 - np.asarray(canvas)).astype("float") / 255.0

def calc_glyph_location(min_arr, int_arr):
    rt1 = []
    for i, it in enumerate(min_arr[:-3]):
        _abs = abs((min_arr[i+1] - it) + (min_arr[i+2] - min_arr[i+1]))
        if _abs > 6.0 and min_arr[i+1] < 10.0:
            rt1.append(i)
    rt2 = []
    for i, it in enumerate(int_arr[:-3]):
        _abs = abs((int_arr[i+1] - it) + (int_arr[i+2] - int_arr[i+1]))
        if _abs > 2.5 and int_arr[i+1] < 15.0:
            rt2.append(i)
    _pop = []
    for i in range(len(rt1)-1):
        if abs(rt1[i] - rt1[i+1]) < 4:
            _pop.append(i)
    for i, it in enumerate(_pop):
        rt1.pop(it - i)
    _pop = []
    for i in range(len(rt2)-1):
        if abs(rt2[i] - rt2[i+1]) < 4:
            _pop.append(i)
    for i, it in enumerate(_pop):
        rt2.pop(it - i)
    return rt1, rt2




'''
This script converts a .h5 Keras model into a Tensorflow .pb file.

Attribution: This script was adapted from https://github.com/amir-abdi/keras_to_tensorflow

MIT License

Copyright (c) 2017 bitbionic

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
'''



from tensorflow.keras.models import load_model
from tensorflow.keras import backend as K

"""
Example:
python convert_model.py --m letter_numeral_model.hdf5 -n 1
"""


def convertGraph( modelPath, outdir, numoutputs, prefix, name):
    '''
    Converts an HD5F file to a .pb file for use with Tensorflow.

    Args:
        modelPath (str): path to the .h5 file
           outdir (str): path to the output directory
       numoutputs (int):
           prefix (str): the prefix of the output aliasing
             name (str):
    Returns:
        None
    '''

    #NOTE: If using Python > 3.2, this could be replaced with os.makedirs( name, exist_ok=True )
    if not os.path.isdir(outdir):
        os.mkdir(outdir)

    K.set_learning_phase(0)

    net_model = load_model(modelPath)

    # Alias the outputs in the model - this sometimes makes them easier to access in TF
    pred = [None]*numoutputs
    pred_node_names = [None]*numoutputs
    for i in range(numoutputs):
        pred_node_names[i] = prefix+'_'+str(i)
        pred[i] = tf.identity(net_model.output[i], name=pred_node_names[i])
    print('Output nodes names are: ', pred_node_names)

    sess = K.get_session()

    # Write the graph in human readable
    f = 'graph_def_for_reference.pb.ascii'
    tf.train.write_graph(sess.graph.as_graph_def(), outdir, f, as_text=True)
    print('Saved the graph definition in ascii format at: ', osp.join(outdir, f))

    # Write the graph in binary .pb file
    from tensorflow.python.framework import graph_util
    from tensorflow.python.framework import graph_io
    sess_graph =  sess.graph.as_graph_def()
    constant_graph = graph_util.convert_variables_to_constants(sess, sess_graph,
     pred_node_names)
    graph_io.write_graph(constant_graph, outdir, name, as_text=False)
    print('Saved the constant graph (ready for inference) at: ', osp.join(outdir, name))
    #print('Run this operation to initialize variables     : ', init.name)


if __name__ == '__main__':

    parser = argparse.ArgumentParser()
    parser.add_argument('--model','-m', dest='model', required=True, help='REQUIRED: The HDF5 Keras model you wish to convert to .pb')
    parser.add_argument('--numout','-n', type=int, dest='num_out', required=True, help='REQUIRED: The number of outputs in the model.')
    parser.add_argument('--outdir','-o', dest='outdir', required=False, default='./', help='The directory to place the output files - default("./")')
    parser.add_argument('--prefix','-p', dest='prefix', required=False, default='k2tfout', help='The prefix for the output aliasing - default("k2tfout")')
    parser.add_argument('--name', dest='name', required=False, default='output_graph.pb', help='The name of the resulting output graph - default("output_graph.pb")')
    args = parser.parse_args()

    convertGraph( args.model, args.outdir, args.num_out, args.prefix, args.name )

