import os
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
package_dir = Path(os.environ.get("DQ_PYTHON_PACKAGE", root / "target" / "python")).resolve()
sys.path.insert(0, str(package_dir))

import dq

assert Path(dq.__file__).resolve().parent == package_dir

source = (root / "tests/assets/source/commerce-platform.d2").read_text(encoding="utf-8")
all_query = (root / "tests/assets/perspectives/all.dl").read_text(encoding="utf-8")
data_path_query = (root / "tests/assets/perspectives/d2-data-path.dl").read_text(
    encoding="utf-8"
)
adversarial_d2 = (root / "tests/assets/source/adversarial-d2.d2").read_text(encoding="utf-8")
adversarial_mermaid = (root / "tests/assets/source/adversarial-mermaid.txt").read_text(
    encoding="utf-8"
)
adversarial_query = (root / "tests/assets/perspectives/adversarial-all.dl").read_text(
    encoding="utf-8"
)
contract_stress = (root / "tests/assets/perspectives/contract-stress.dl").read_text(
    encoding="utf-8"
)

assert dq.transform(source, all_query, "d2", "mermaid") == (
    root / "tests/assets/expect/d2-all.mmd"
).read_text(encoding="utf-8")

document = dq.Document(source, "d2")
assert document.transform(data_path_query, "d2") == (
    root / "tests/assets/expect/d2-data-path.d2"
).read_text(encoding="utf-8")

assert dq.transform(adversarial_d2, adversarial_query, "d2", "mermaid") == (
    root / "tests/assets/expect/adversarial-d2-all.mmd"
).read_text(encoding="utf-8")

assert dq.transform(adversarial_d2, contract_stress, "d2", "d2") == (
    root / "tests/assets/expect/contract-stress.d2"
).read_text(encoding="utf-8")

adversarial_document = dq.Document(adversarial_d2, "d2")
assert adversarial_document.transform(contract_stress, "mermaid") == (
    root / "tests/assets/expect/contract-stress.mmd"
).read_text(encoding="utf-8")

mermaid_document = dq.Document(adversarial_mermaid, "mermaid")
assert mermaid_document.transform(adversarial_query, "d2") == (
    root / "tests/assets/expect/adversarial-mermaid-all.d2"
).read_text(encoding="utf-8")


def assert_value_error(operation, message):
    try:
        operation()
    except ValueError as error:
        assert str(error) == message
    else:
        raise AssertionError("invalid format was accepted")


assert_value_error(
    lambda: dq.transform(source, all_query, "dot", "d2"),
    "input_format must be d2|mermaid",
)
assert_value_error(
    lambda: dq.transform(source, all_query, "d2", "json"),
    "output_format must be d2|mermaid",
)

print("Python PyO3 fixture smoke passed")
