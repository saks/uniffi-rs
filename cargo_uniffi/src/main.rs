/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::Result;

fn main() -> Result<()> {
    let matches = cargo_uniffi::build_arg_matcher().get_matches();
    cargo_uniffi::execute_command(matches)?;
    Ok(())
}
