Voxel Engine

Docs



Voxel representantion on the GPU:

1. two layers of buffers:
    -chunk buffer
        one chunk holds the structure within the region
        points to blocks in <block buffer>
    -block buffer


+----
|
|
|
