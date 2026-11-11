#!/usr/bin/env bash
# Draw the gallery with the current CLI, using the same paths as its readers.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

gallery=crates/kaalang/examples/gallery
drawn=()
while read -r kind item destination option; do
  drawn+=("$gallery/$destination.svg")
  arguments=(diagram -p kaalang "$kind" "$item" --target x86_64-unknown-linux-gnu
    -o "$gallery/$destination.svg")
  if [[ -n "$option" ]]; then
    arguments+=("$option")
  fi
  cargo run --quiet -p kaalang-cli -- "${arguments[@]}"
done <<'DIAGRAMS'
--flow binary_search::binary_search binary_search/binary_search
--flow binary_search::binary_search binary_search/binary_search_collapsed --collapse-cycles
--flow binary_search::binary_search_swapped binary_search/binary_search_swapped
--flow binary_search::binary_search_swapped binary_search/binary_search_swapped_collapsed --collapse-cycles
--flow fizzbuzz::fizzbuzz fizzbuzz/fizzbuzz
--flow kmp_search::kmp_search kmp_search/kmp_search
--flow kmp_search::prefix_table kmp_search/prefix_table
--flow kmp_search::prefix_table kmp_search/prefix_table_collapsed --collapse-cycles
--flow logical_formulas::and::and logical_formulas/and
--flow logical_formulas::inverted_and::inverted_and logical_formulas/inverted_and
--flow logical_formulas::inverted_or::inverted_or logical_formulas/inverted_or
--flow logical_formulas::or::or logical_formulas/or
--flow loops::do_while::do_while loops/do_while
--flow loops::do_while::do_while loops/do_while_collapsed --collapse-cycles
--flow loops::endless::endless loops/endless
--flow loops::endless::endless loops/endless_collapsed --collapse-cycles
--flow loops::for_each::for_each loops/for_each
--flow loops::for_each::for_each loops/for_each_collapsed --collapse-cycles
--flow loops::iterator_loop::iterator_loop loops/iterator_loop
--flow loops::iterator_loop::iterator_loop loops/iterator_loop_collapsed --collapse-cycles
--flow loops::middle_exit::middle_exit loops/middle_exit
--flow loops::middle_exit::middle_exit loops/middle_exit_collapsed --collapse-cycles
--flow loops::while_loop::while_loop loops/while_loop
--flow loops::while_loop::while_loop loops/while_loop_collapsed --collapse-cycles
--type order::Backlog order/backlog
--type order::Customer order/customer
--type order::CustomerId order/customer_id
--type order::Hold order/hold
--type order::Line order/line
--type order::Money order/money
--type order::Order order/order
--type order::OrderEvent order/order_event
--type order::OrderNumber order/order_number
--type order::Payment order/payment
--type order::Product order/product
--type order::Receipt order/receipt
--type order::Shipment order/shipment
--type order::Sku order/sku
--flow sorting::bubble_sort::bubble_sort sorting/bubble_sort
--flow sorting::bubble_sort::bubble_sort sorting/bubble_sort_collapsed --collapse-cycles
--flow sorting::quick_sort::quick_sort sorting/quick_sort
--flow sorting::quick_sort::quick_sort sorting/quick_sort_collapsed --collapse-cycles
--flow swap::swap swap/swap
DIAGRAMS

# A diagram no line draws any more is removed, so the diff shows it.
while IFS= read -r -d '' file; do
  if [[ ! " ${drawn[*]} " =~ " $file " ]]; then
    rm "$file"
  fi
done < <(find "$gallery" -name '*.svg' -print0)
