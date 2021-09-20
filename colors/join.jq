# https://stackoverflow.com/questions/39830426/join-two-json-files-based-on-common-key-with-jq-utility-or-alternative-way-from

# hashJoin(a1; a2; field) expects a1 and a2 to be arrays of JSON objects
# and that for each of the objects, the field value is a string.
# A relational join is performed on "field".

def hashJoin(a1; a2; field):
  # hash phase:
  (reduce a1[] as $o ({};  . + { ($o | field | tostring): {name: $o.name} } )) as $h1
  | (reduce a2[] as $o ({};  . + { ($o | field | tostring): {index: $o.colorId } } )) as $h2
  # join phase:
  | reduce ($h1|keys[]) as $key
      ([]; if $h2|has($key) then . + [ $h1[$key] + $h2[$key] ] else . end);

hashJoin( $simpler; $xterm; .colorId)[]