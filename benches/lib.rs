// Copyright 2017 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement.  This, along with the Licenses can be
// found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

// For explanation of lint checks, run `rustc -W help` or see
// https://github.com/maidsafe/QA/blob/master/Documentation/Rust%20Lint%20Checks.md

#![forbid(bad_style, exceeding_bitshifts, mutable_transmutes, no_mangle_const_items,
          unknown_crate_types, warnings)]
#![deny(deprecated, improper_ctypes, missing_docs,
        non_shorthand_field_patterns, overflowing_literals, plugin_as_library,
        private_no_mangle_fns, private_no_mangle_statics, stable_features, unconditional_recursion,
        unknown_lints, unsafe_code, unused, unused_allocation, unused_attributes,
        unused_comparisons, unused_features, unused_parens, while_true)]
#![warn(trivial_casts, trivial_numeric_casts, unused_extern_crates, unused_import_braces,
        unused_qualifications, unused_results)]
#![allow(box_pointers, fat_ptr_transmutes, missing_copy_implementations,
         missing_debug_implementations, variant_size_differences)]

#![feature(test)]

extern crate rand;
extern crate test;
extern crate cask;
extern crate maidsafe_utilities;

use cask::Cask;

use rand::Rng;
use test::Bencher;
use maidsafe_utilities::serialisation;

fn generate_random_bytes(size: u64) -> Vec<u8> {
    rand::thread_rng()
        .gen_iter()
        .take(size as usize)
        .collect()
}

// #[bench]
// fn bench_write(b: &mut Bencher) {
//     let one_mb = 1024 * 1024;
//     let data = generate_random_bytes(one_mb);

//     let cask = Cask::open("test.db", false);

//     b.iter(|| {
//         let key = generate_random_bytes(4);
//         let serialised_value = match serialisation::serialise(&data) {
//           Ok(result) => result,
//           Err(_) => return,
//         };
//         let _ = cask.put(key, serialised_value);
//     });
// }

// #[bench]
// fn bench_serialise(b: &mut Bencher) {
//     let one_mb = 1024 * 1024;
//     let data = generate_random_bytes(one_mb);

//     b.iter(|| {
//         let _ = serialisation::serialise(&data);
//     });
// }

// #[bench]
// fn bench_read(b: &mut Bencher) {
//     let one_mb = 1024 * 1024;
//     let mut data = generate_random_bytes(one_mb);

//     let cask = Cask::open("test3.db", false);

//     let key_0 = generate_random_bytes(4);

//     let _ = cask.put(key_0.clone(), data.clone());

//     for i in 0..3096 {
//         let key = generate_random_bytes(4);
//         data[i] = generate_random_bytes(1)[0];
//         let _ = cask.put(key, data.clone());
//     }


//     let key_n = generate_random_bytes(4);
//     let _ = cask.put(key_n.clone(), data.clone());

//     b.iter(|| {
//         let _ = cask.get(key_0.clone());
//         let _ = cask.get(key_n.clone());
//     });
// }

// #[bench]
// fn bench_read(b: &mut Bencher) {
//     let one_mb = 1024 * 1024;
//     let data = generate_random_bytes(one_mb);

//     let cask = Cask::open("test1.db", true);

//     let key_0 = generate_random_bytes(4);

//     let _ = cask.put(key_0.clone(), data.clone());

//     b.iter(|| {
//         let _ = cask.get(key_0.clone());
//     });
// }

#[bench]
fn bench_read(b: &mut Bencher) {
    let one_mb = 1024 * 1024;
    let data = generate_random_bytes(one_mb);
    let serialised_value = match serialisation::serialise(&data) {
      Ok(result) => result,
      Err(_) => return,
    };

    let cask = Cask::open("test4.db", false);

    let key_0 = generate_random_bytes(4);

    let _ = cask.put(key_0.clone(), serialised_value);

    b.iter(|| {
        let result = cask.get(key_0.clone()).unwrap();
        let _ = serialisation::deserialise::<Vec<u8>>(&result);
    });
}
