# Local face detection and recognition models

`seeta_fd_frontal_v1.0.bin` is the face **detection** model distributed with
[rustface](https://github.com/atomashpolskiy/rustface/tree/fa3d5b5d576c91d1c8a099af381e0f0906a0d3a3),
pinned to commit `fa3d5b5d576c91d1c8a099af381e0f0906a0d3a3`.
The upstream BSD 2-Clause license is included in `LICENSE-rustface`.

The detector is embedded in the desktop binary. Processing is CPU-only and offline,
using at most a 960px local display image, one worker, and score threshold 4.5.
Scores are detector scores, not calibrated probabilities.

`mobile_facenet/` contains Qualcomm's float ONNX export of MobileFaceNet, downloaded
from the [Qualcomm MobileFaceNet model card](https://huggingface.co/qualcomm/MobileFaceNet)
(release `v0.62.2`, Apache-2.0). It accepts two 112 × 112 RGB inputs and produces
128-dimensional embeddings. The app runs it through the pure-Rust, CPU-only tract
runtime. Face embeddings are built in memory for one background job and are never
persisted. Only confirmed solo collections with one linked person and one detected
face become reference samples.

Side profiles, obscured faces, small faces, background people, and unaligned crops
can lead to incorrect results; every type and identity suggestion requires review.
