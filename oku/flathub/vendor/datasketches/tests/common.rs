// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::path::PathBuf;

#[allow(dead_code)] // false-positive
pub fn test_data(name: &str) -> PathBuf {
    const TEST_DATA_DIR: &str = "tests/test_data";

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(TEST_DATA_DIR)
        .join(name)
}

pub fn serialization_test_data(sub_dir: &str, name: &str) -> PathBuf {
    const SERDE_TEST_DATA_DIR: &str = "tests/serialization_test_data";

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(SERDE_TEST_DATA_DIR)
        .join(sub_dir)
        .join(name);

    if !path.exists() {
        panic!(
            r#"serialization test data file not found: {}

            Please ensure test data files are present in the repository. Generally, you can
            run the following commands from the project root to regenerate the test data files
            if they are missing:

            $ ./tools/generate_serialization_test_data.py
        "#,
            path.display(),
        );
    }

    path
}
