instance="$1"
count="${2:-1}"

sleep 1
echo '['
for n in `seq 1 1 $count`; do
  echo '{"name":null,"instance":"'$instance'","button":'$n',"modifiers":[],"x":11,"y":12,"relative_x":15,"relative_y":16,"output_x":9,"output_y":8,"width":13,"height":14}'
done
sleep 999