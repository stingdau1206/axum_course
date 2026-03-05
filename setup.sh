# Danh sách tên các module
modules=(
    "intro" "routing" "extractors" "responses" "state" 
    "middleware" "errors" "database" "auth" "advanced" 
    "testing" "production"
)

# Chạy vòng lặp để tạo các crate (dạng library)
for i in "${!modules[@]}"; do
    index=$(printf "%02d" $((i+1)))
    name="module-$index-${modules[$i]}"
    # cargo new "$name"
    rm -rf $name/.git
done