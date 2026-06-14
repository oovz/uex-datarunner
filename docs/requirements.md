# UEX Datarunner Requirements

## Functional Requirements

1. The first launch must show Settings until setup is complete.
2. Users must be able to reopen Settings from the main screen at any time.
3. Settings must allow editing:
   - UEX secret key
   - Star Citizen screenshot folder
   - Foundry Local AI model alias
   - game version
   - delete-after-submit behavior
4. Settings must validate:
   - UEX secret key against UEX `/user`
   - Foundry Local model availability and image input capability
   - screenshot folder presence before processing
5. The main screen must let users:
   - refresh pending screenshots
   - run OCR on pending screenshots
   - choose/search a UEX commodity terminal
   - review extracted commodity rows
   - manually adjust commodity name, commodity ID, market side, price, SCU, status, and cargo sizes when present
   - remove rows before submission
   - submit accepted rows to UEX
6. The review table must show one side per row. It must not show separate buy and sell column groups for the same screenshot.
7. A single screenshot submission must not mix buy-side and sell-side rows.
8. API testing mode must not be exposed as a user-facing choice. Test runs may configure non-production submission internally.
9. The desktop title bar must be draggable on Windows.
10. The app window should use a slim vertical layout without page-level horizontal scrollbars or oversized table content.

## AI Extraction Requirements

1. The OCR model must be configurable between CUDA-backed Foundry Local aliases, currently `qwen3.5-4b` and `qwen-3-vl-4b-instruct`.
2. The app must use Foundry Local for local inference.
3. The app must not call legacy Windows OCR, TextExtractor, or Windows AI TextRecognizer APIs.
4. The extraction prompt must prefer structured output.
5. Missing commodity fields must stay blank. The model and parser must not invent values.
6. Allowed cargo size values are `1`, `2`, `4`, `8`, `16`, `24`, and `32`.
7. CPU-only, WebGPU, and unavailable non-CUDA execution paths are unsupported.

## UEX Submission Requirements

1. The UEX secret key must only be sent as the `secret-key` header by the backend.
2. The secret key must never be placed in a `data_submit` payload body.
3. Commodity payloads must use `type: "commodity"`.
4. `is_production` must be `1` for normal use and `0` only for controlled test mode.
5. Each UEX price row may include only one side of price data:
   - buy: `price_buy`, `scu_buy`, `status_buy`
   - sell: `price_sell`, `scu_sell`, `status_sell`
6. Commodity status must be an integer from `1` through `7`.
7. Payloads must contain at most 500 price rows.
8. Rows from different screenshots must be submitted screenshot-first. If one screenshot contains buy-side rows and another contains sell-side rows, submit one payload per screenshot with that screenshot attached.

## Non-Functional Requirements

1. The UI should follow default shadcn-style dark neutral styling with a restrained Star Citizen accent only where useful.
2. Form controls need labels, visible focus states, autocomplete attributes where appropriate, and no blocked paste.
3. Icon-only buttons need accessible labels.
4. Expensive or risky behavior must be guarded by automated tests.
5. Credentials and local screenshots must remain local except for explicit UEX submission.
