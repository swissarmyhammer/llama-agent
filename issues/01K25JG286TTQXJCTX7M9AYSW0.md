Oops -- you aren't downloading multiple files


take a look at https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/tree/main

and 

https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/tree/main/BF16


You are failing to get the second file before you try to load


 cargo run --package llama-agent-cli -- --model unsloth/Qwen3-30B-A3B-GGUF  --prompt "What is an apple?" --limit 64
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.09s
     Running `target/debug/llama-agent-cli --model unsloth/Qwen3-30B-A3B-GGUF --prompt 'What is an apple?' --limit 64`
2025-08-08T19:04:13.772434Z  WARN llama_agent::model: Download attempt 1 failed for 'BF16/Qwen3-30B-A3B-BF16-00001-of-00002.gguf': request error: HTTP status server error (500 Internal Server Error) for url (<https://cas-bridge.xethub.hf.co/xet-bridge-us/680f87393952fce74921f3d9/6c4fcb075e3a8d1fcc10d5e2ea002a809aa01db0a47040cdf64b77fd5599a650?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Content-Sha256=UNSIGNED-PAYLOAD&X-Amz-Credential=cas%2F20250808%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20250808T185608Z&X-Amz-Expires=3600&X-Amz-Signature=4e402d738580856b3eb4af27df9f4fb88c83f455dec8e3b1fe7aeac0569b782f&X-Amz-SignedHeaders=host&X-Xet-Cas-Uid=6463f0187572c66a8e63984a&response-content-disposition=inline%3B+filename*%3DUTF-8%27%27Qwen3-30B-A3B-BF16-00001-of-00002.gguf%3B+filename%3D%22Qwen3-30B-A3B-BF16-00001-of-00002.gguf%22%3B&x-id=GetObject&Expires=1754682968&Policy=eyJTdGF0ZW1lbnQiOlt7IkNvbmRpdGlvbiI6eyJEYXRlTGVzc1RoYW4iOnsiQVdTOkVwb2NoVGltZSI6MTc1NDY4Mjk2OH19LCJSZXNvdXJjZSI6Imh0dHBzOi8vY2FzLWJyaWRnZS54ZXRodWIuaGYuY28veGV0LWJyaWRnZS11cy82ODBmODczOTM5NTJmY2U3NDkyMWYzZDkvNmM0ZmNiMDc1ZTNhOGQxZmNjMTBkNWUyZWEwMDJhODA5YWEwMWRiMGE0NzA0MGNkZjY0Yjc3ZmQ1NTk5YTY1MCoifV19&Signature=PykTRauWvl6%7EWQA3mEVu-CJjCnTJ49xNO5ZTlbfLQ0CFRoshl1cdbZ2FH4sN2znOVt5rgeAIsYTBgFeAnVUsgwtQXKNgJutw4PoJeASV9U-xUWjg%7EAptSRUxcH6ZW%7EdNDG8ACYEE30WhnmezfxLLAYI9gcM238C%7EGaAW7grtOahhKmV3LIxMNPptIoq99R3Qpq5h8Y4MyoiM1x44RG3dYg0rzz7jnhSJTqTHmgRF7fLeLFkBuI0QbWzTP45UIiATZYhFw1lkD-SXE78wH8hkzj3lOQLDOpHUreeojTpIp3KndG4SrtQuDRoJfAf59uPAfLDulUZfh-r3RGobFk0rJw__&Key-Pair-Id=K2L8F4GPSG1IFC>). Retrying in 1000ms...
..B-A3B-BF16-00001-of-00002.gguf [00:08:42] [██████████████████████████████████████████████████████████████] 46.28 GiB/46.28 GiB 90.71 MiB/s (0s)Model Error: Failed to initialize agent: Model error: Model loading failed: Failed to load downloaded model from /Users/wballard/.cache/huggingface/hub/models--unsloth--Qwen3-30B-A3B-GGUF/snapshots/d5b1d57bd0b504ac62ae6c725904e96ef228dc74/BF16/Qwen3-30B-A3B-BF16-00001-of-00002.gguf: null result from llama cpp
🔧 Check available memory (4-8GB needed), verify GGUF file integrity, ensure compatible llama.cpp version
💡 Check model file exists, is valid GGUF format, and sufficient memory is available


## Proposed Solution

After analyzing the issue and examining the HuggingFace repository, I've identified the root cause:

1. **Problem**: The current `auto_detect_hf_model_file` function only returns the first BF16 file it finds (`Qwen3-30B-A3B-BF16-00001-of-00002.gguf`), but multi-part GGUF files require all parts to be downloaded before loading.

2. **Solution**: Implement multi-part GGUF file detection and download:
   - Detect when a GGUF file is part of a multi-part set (contains pattern like `00001-of-00002`)
   - Extract the total number of parts from the filename
   - Download all parts in the correct order
   - Ensure all parts are downloaded before attempting to load the model
   - Modify the model loading to handle multi-part files correctly

3. **Implementation Steps**:
   - Create a function to detect multi-part GGUF files
   - Create a function to download all parts of a multi-part GGUF file
   - Modify the `auto_detect_hf_model_file` function to handle multi-part files
   - Update the download logic to download all required parts
   - Handle the model loading to use the first part (as llama.cpp typically loads from the first part)

4. **Test Plan**:
   - Test with the failing model `unsloth/Qwen3-30B-A3B-GGUF` 
   - Ensure both parts are downloaded before loading
   - Verify the model loads successfully after all parts are present