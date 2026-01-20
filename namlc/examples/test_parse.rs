///
/// Comprehensive Parser Test
///
/// This file tests the naml parser with a complex, real-world-like codebase
/// that exercises all language features: interfaces, structs, enums, generics,
/// async/await, error handling, lambdas, and more.
///

use std::time::Instant;

fn main() {
    let source = r#"
// =============================================================================
// Generic Collections Library
// =============================================================================

interface Comparable<T> {
    fn compare(other: T) -> int;
    fn equals(other: T) -> bool;
}

interface Hashable {
    fn hash() -> uint;
}

interface Iterator<T> {
    fn next() -> option<T>;
    fn has_next() -> bool;
}

interface Iterable<T> {
    fn iter() -> Iterator<T>;
}

interface Collection<T> {
    fn size() -> int;
    fn is_empty() -> bool;
    fn contains(item: T) -> bool;
    fn add(item: T);
    fn remove(item: T) -> bool;
    fn clear();
}

interface Serializable {
    fn to_json() -> string;
    fn to_bytes() -> bytes;
}


// =============================================================================
// Error Types
// =============================================================================

exception NetworkError {
    message: string,
    code: int,
    retry_after: option<int>
}

exception ValidationError {
    field: string,
    message: string,
    value: string
}

exception DatabaseError {
    query: string,
    message: string,
    error_code: int
}

exception AuthenticationError {
    user_id: string,
    reason: string,
    timestamp: int
}

// =============================================================================
// Domain Models
// =============================================================================

pub struct UserId implements Comparable<UserId>, Hashable {
    pub value: string
}

pub fn (self: UserId) compare(other: UserId) -> int {
    if (self.value == other.value) {
        return 0;
    }
    if (self.value < other.value) {
        return -1;
    }
    return 1;
}

pub fn (self: UserId) equals(other: UserId) -> bool {
    return self.value == other.value;
}

pub fn (self: UserId) hash() -> uint {
    var h: uint = 0;
    for (i, c in self.value) {
        h = h * 31 + (c as uint);
    }
    return h;
}

pub struct Email implements Comparable<Email> {
    pub local: string,
    pub domain: string
}

pub fn (self: Email) to_string() -> string {
    return self.local + "@" + self.domain;
}

pub fn (self: Email) compare(other: Email) -> int {
    var self_str: string = self.to_string();
    var other_str: string = other.to_string();
    if (self_str == other_str) {
        return 0;
    }
    if (self_str < other_str) {
        return -1;
    }
    return 1;
}

pub fn (self: Email) equals(other: Email) -> bool {
    return self.local == other.local && self.domain == other.domain;
}

pub struct Address {
    pub street: string,
    pub city: string,
    pub state: string,
    pub zip: string,
    pub country: string
}

pub fn (self: Address) format() -> string {
    return self.street + "\n" + self.city + ", " + self.state + " " + self.zip + "\n" + self.country;
}

pub fn (self: Address) is_valid() -> bool {
    return self.street != "" && self.city != "" && self.zip != "";
}

pub struct PhoneNumber {
    pub country_code: string,
    pub area_code: string,
    pub number: string,
    pub extension: option<string>
}

pub fn (self: PhoneNumber) format() -> string {
    var base: string = "+" + self.country_code + " (" + self.area_code + ") " + self.number;
    var ext: string = self.extension else {
        return base;
    }
    return base + " ext. " + ext;
}

enum UserStatus {
    Active,
    Inactive,
    Suspended(string),
    PendingVerification(string),
    Deleted(int)
}

enum UserRole {
    Admin,
    Moderator,
    User,
    Guest,
    Custom(string, [string])
}

pub struct User implements Serializable {
    pub id: UserId,
    pub email: Email,
    pub username: string,
    pub password_hash: string,
    pub first_name: option<string>,
    pub last_name: option<string>,
    pub address: option<Address>,
    pub phone: option<PhoneNumber>,
    pub status: UserStatus,
    pub roles: [UserRole],
    pub metadata: map<string, string>,
    pub created_at: int,
    pub updated_at: int,
    pub last_login: option<int>
}

pub fn (self: User) full_name() -> string {
    var first: string = self.first_name.or_default("");
    var last: string = self.last_name.or_default("");
    if (first == "" && last == "") {
        return self.username;
    }
    if (first == "") {
        return last;
    }
    if (last == "") {
        return first;
    }
    return first + " " + last;
}

pub fn (self: User) is_active() -> bool {
    switch (self.status) {
        case Active: {
            return true;
        }
        default: {
            return false;
        }
    }
}

pub fn (self: User) has_role(role: UserRole) -> bool {
    for (r in self.roles) {
        if (r == role) {
            return true;
        }
    }
    return false;
}

pub fn (self: User) is_admin() -> bool {
    return self.has_role(UserRole::Admin);
}

pub fn (self: User) to_json() -> string {
    return "{\"id\": \"" + self.id.value + "\", \"email\": \"" + self.email.to_string() + "\"}";
}

pub fn (self: User) to_bytes() -> bytes {
    return self.to_json() as bytes;
}

// =============================================================================
// Generic Data Structures
// =============================================================================

pub struct LinkedListNode<T> {
    pub value: T,
    pub next: option<LinkedListNode<T>>
}

pub struct LinkedList<T> implements Collection<T> {
    pub head: option<LinkedListNode<T>>,
    pub tail: option<LinkedListNode<T>>,
    pub length: int
}

pub fn (self: LinkedList<T>) size() -> int {
    return self.length;
}

pub fn (self: LinkedList<T>) is_empty() -> bool {
    return self.length == 0;
}

pub fn (mut self: LinkedList<T>) add(item: T) {
    var node: LinkedListNode<T> = LinkedListNode { value: item, next: none };
    if (self.head.is_none()) {
        self.head = some(node);
        self.tail = some(node);
    } else {
        var tail_node: LinkedListNode<T> = self.tail else {
            return;
        }
        tail_node.next = some(node);
        self.tail = some(node);
    }
    self.length = self.length + 1;
}

pub fn (self: LinkedList<T>) contains(item: T) -> bool {
    var current: option<LinkedListNode<T>> = self.head;
    while (current.is_some()) {
        var node: LinkedListNode<T> = current else {
            break;
        }
        if (node.value == item) {
            return true;
        }
        current = node.next;
    }
    return false;
}

pub fn (mut self: LinkedList<T>) remove(item: T) -> bool {
    if (self.head.is_none()) {
        return false;
    }
    var head_node: LinkedListNode<T> = self.head else {
        return false;
    }
    if (head_node.value == item) {
        self.head = head_node.next;
        self.length = self.length - 1;
        return true;
    }
    var current: option<LinkedListNode<T>> = self.head;
    while (current.is_some()) {
        var curr_node: LinkedListNode<T> = current else {
            break;
        }
        if (curr_node.next.is_none()) {
            break;
        }
        var next_node: LinkedListNode<T> = curr_node.next else {
            break;
        }
        if (next_node.value == item) {
            curr_node.next = next_node.next;
            self.length = self.length - 1;
            return true;
        }
        current = curr_node.next;
    }
    return false;
}

pub fn (mut self: LinkedList<T>) clear() {
    self.head = none;
    self.tail = none;
    self.length = 0;
}

pub struct TreeNode<T> {
    pub value: T,
    pub left: option<TreeNode<T>>,
    pub right: option<TreeNode<T>>,
    pub height: int
}

pub fn (self: TreeNode<T>) is_leaf() -> bool {
    return self.left.is_none() && self.right.is_none();
}

pub fn (self: TreeNode<T>) balance_factor() -> int {
    var left_height: int = 0;
    var right_height: int = 0;
    if (self.left.is_some()) {
        var left_node: TreeNode<T> = self.left else {
            return 0;
        }
        left_height = left_node.height;
    }
    if (self.right.is_some()) {
        var right_node: TreeNode<T> = self.right else {
            return left_height;
        }
        right_height = right_node.height;
    }
    return left_height - right_height;
}

pub struct BinarySearchTree<T: Comparable<T>> {
    pub root: option<TreeNode<T>>,
    pub size: int
}

pub fn (mut self: BinarySearchTree<T>) insert(value: T) {
    var node: TreeNode<T> = TreeNode { value: value, left: none, right: none, height: 1 };
    if (self.root.is_none()) {
        self.root = some(node);
    } else {
        var root_node: TreeNode<T> = self.root else {
            return;
        }
        self.insert_recursive(root_node, value);
    }
    self.size = self.size + 1;
}

fn (self: BinarySearchTree<T>) insert_recursive(current: TreeNode<T>, value: T) {
    var cmp: int = value.compare(current.value);
    if (cmp < 0) {
        if (current.left.is_none()) {
            current.left = some(TreeNode { value: value, left: none, right: none, height: 1 });
        } else {
            var left_node: TreeNode<T> = current.left else {
                return;
            }
            self.insert_recursive(left_node, value);
        }
    } else {
        if (current.right.is_none()) {
            current.right = some(TreeNode { value: value, left: none, right: none, height: 1 });
        } else {
            var right_node: TreeNode<T> = current.right else {
                return;
            }
            self.insert_recursive(right_node, value);
        }
    }
    self.update_height(current);
}

fn (self: BinarySearchTree<T>) update_height(node: TreeNode<T>) {
    var left_height: int = 0;
    var right_height: int = 0;
    if (node.left.is_some()) {
        var left_node: TreeNode<T> = node.left else {
            return;
        }
        left_height = left_node.height;
    }
    if (node.right.is_some()) {
        var right_node: TreeNode<T> = node.right else {
            return;
        }
        right_height = right_node.height;
    }
    if (left_height > right_height) {
        node.height = left_height + 1;
    } else {
        node.height = right_height + 1;
    }
}

pub fn (self: BinarySearchTree<T>) find(value: T) -> option<T> {
    return self.find_recursive(self.root, value);
}

fn (self: BinarySearchTree<T>) find_recursive(current: option<TreeNode<T>>, value: T) -> option<T> {
    if (current.is_none()) {
        return none;
    }
    var node: TreeNode<T> = current else {
        return none;
    }
    var cmp: int = value.compare(node.value);
    if (cmp == 0) {
        return some(node.value);
    }
    if (cmp < 0) {
        return self.find_recursive(node.left, value);
    }
    return self.find_recursive(node.right, value);
}

// =============================================================================
// Async HTTP Client
// =============================================================================

pub struct HttpHeaders {
    pub headers: map<string, string>
}

pub fn (mut self: HttpHeaders) set(name: string, value: string) {
    self.headers[name] = value;
}

pub fn (self: HttpHeaders) get(name: string) -> option<string> {
    if (self.headers[name].is_some()) {
        return some(self.headers[name].or_default(""));
    }
    return none;
}

pub struct HttpRequest {
    pub method: string,
    pub url: string,
    pub headers: HttpHeaders,
    pub body: option<bytes>,
    pub timeout_ms: int
}

pub struct HttpResponse {
    pub status_code: int,
    pub status_text: string,
    pub headers: HttpHeaders,
    pub body: bytes,
    pub elapsed_ms: int
}

pub fn (self: HttpResponse) is_success() -> bool {
    return self.status_code >= 200 && self.status_code < 300;
}

pub fn (self: HttpResponse) is_redirect() -> bool {
    return self.status_code >= 300 && self.status_code < 400;
}

pub fn (self: HttpResponse) is_client_error() -> bool {
    return self.status_code >= 400 && self.status_code < 500;
}

pub fn (self: HttpResponse) is_server_error() -> bool {
    return self.status_code >= 500;
}

interface HttpClient {
    async fn get(url: string) -> HttpResponse throws NetworkError;
    async fn post(url: string, body: bytes) -> HttpResponse throws NetworkError;
    async fn put(url: string, body: bytes) -> HttpResponse throws NetworkError;
    async fn delete(url: string) -> HttpResponse throws NetworkError;
    async fn request(req: HttpRequest) -> HttpResponse throws NetworkError;
}

pub struct SimpleHttpClient implements HttpClient {
    pub base_url: string,
    pub default_headers: HttpHeaders,
    pub timeout_ms: int,
    pub retry_count: int,
    pub retry_delay_ms: int
}

pub async fn (self: SimpleHttpClient) get(url: string) -> HttpResponse throws NetworkError {
    var req: HttpRequest = HttpRequest {
        method: "GET",
        url: self.base_url + url,
        headers: self.default_headers,
        body: none,
        timeout_ms: self.timeout_ms
    };
    return await self.request(req);
}

pub async fn (self: SimpleHttpClient) post(url: string, body: bytes) -> HttpResponse throws NetworkError {
    var req: HttpRequest = HttpRequest {
        method: "POST",
        url: self.base_url + url,
        headers: self.default_headers,
        body: some(body),
        timeout_ms: self.timeout_ms
    };
    return await self.request(req);
}

pub async fn (self: SimpleHttpClient) put(url: string, body: bytes) -> HttpResponse throws NetworkError {
    var req: HttpRequest = HttpRequest {
        method: "PUT",
        url: self.base_url + url,
        headers: self.default_headers,
        body: some(body),
        timeout_ms: self.timeout_ms
    };
    return await self.request(req);
}

pub async fn (self: SimpleHttpClient) delete(url: string) -> HttpResponse throws NetworkError {
    var req: HttpRequest = HttpRequest {
        method: "DELETE",
        url: self.base_url + url,
        headers: self.default_headers,
        body: none,
        timeout_ms: self.timeout_ms
    };
    return await self.request(req);
}

pub async fn (self: SimpleHttpClient) request(req: HttpRequest) -> HttpResponse throws NetworkError {
    var attempts: int = 0;
    while (attempts < self.retry_count) {
        var response: HttpResponse = await self.do_request(req);
        if (response.is_success()) {
            return response;
        }
        if (response.is_client_error()) {
            throw NetworkError {
                message: "Client error: " + response.status_text,
                code: response.status_code,
                retry_after: none
            };
        }
        attempts = attempts + 1;
        if (attempts < self.retry_count) {
            await sleep(self.retry_delay_ms);
        }
    }
    throw NetworkError {
        message: "Max retries exceeded",
        code: -1,
        retry_after: some(self.retry_delay_ms)
    };
}

async fn (self: SimpleHttpClient) do_request(req: HttpRequest) -> HttpResponse {
    return HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: HttpHeaders { headers: {} },
        body: "" as bytes,
        elapsed_ms: 0
    };
}

extern fn sleep(ms: int) -> promise<unit>;

// =============================================================================
// Database Repository Pattern
// =============================================================================

interface Repository<T, ID> {
    async fn find_by_id(id: ID) -> option<T> throws DatabaseError;
    async fn find_all() -> [T] throws DatabaseError;
    async fn save(entity: T) -> T throws DatabaseError;
    async fn delete(id: ID) -> bool throws DatabaseError;
    async fn exists(id: ID) -> bool throws DatabaseError;
    async fn count() -> int throws DatabaseError;
}

pub struct UserRepository implements Repository<User, UserId> {
    pub connection_string: string,
    pub table_name: string
}

pub async fn (self: UserRepository) find_by_id(id: UserId) -> option<User> throws DatabaseError {
    var query: string = "SELECT * FROM " + self.table_name + " WHERE id = ?";
    var result: [map<string, string>] = await self.execute_query(query, [id.value]);
    if (result.length == 0) {
        return none;
    }
    return some(self.map_row_to_user(result[0]));
}

pub async fn (self: UserRepository) find_all() -> [User] throws DatabaseError {
    var query: string = "SELECT * FROM " + self.table_name;
    var result: [map<string, string>] = await self.execute_query(query, []);
    var users: [User] = [];
    for (row in result) {
        users = users + [self.map_row_to_user(row)];
    }
    return users;
}

pub async fn (self: UserRepository) save(entity: User) -> User throws DatabaseError {
    var existing: option<User> = await self.find_by_id(entity.id);
    if (existing.is_some()) {
        return await self.update(entity);
    }
    return await self.insert(entity);
}

async fn (self: UserRepository) insert(entity: User) -> User throws DatabaseError {
    var query: string = "INSERT INTO " + self.table_name + " (id, email, username) VALUES (?, ?, ?)";
    await self.execute_update(query, [entity.id.value, entity.email.to_string(), entity.username]);
    return entity;
}

async fn (self: UserRepository) update(entity: User) -> User throws DatabaseError {
    var query: string = "UPDATE " + self.table_name + " SET email = ?, username = ? WHERE id = ?";
    await self.execute_update(query, [entity.email.to_string(), entity.username, entity.id.value]);
    return entity;
}

pub async fn (self: UserRepository) delete(id: UserId) -> bool throws DatabaseError {
    var query: string = "DELETE FROM " + self.table_name + " WHERE id = ?";
    var affected: int = await self.execute_update(query, [id.value]);
    return affected > 0;
}

pub async fn (self: UserRepository) exists(id: UserId) -> bool throws DatabaseError {
    var query: string = "SELECT COUNT(*) FROM " + self.table_name + " WHERE id = ?";
    var result: [map<string, string>] = await self.execute_query(query, [id.value]);
    var count_str: string = result[0]["count"].or_default("0");
    return count_str > "0";
}

pub async fn (self: UserRepository) count() -> int throws DatabaseError {
    var query: string = "SELECT COUNT(*) as cnt FROM " + self.table_name;
    var result: [map<string, string>] = await self.execute_query(query, []);
    var cnt_str: string = result[0]["cnt"].or_default("0");
    return cnt_str as int;
}

async fn (self: UserRepository) execute_query(query: string, params: [string]) -> [map<string, string>] throws DatabaseError {
    return [];
}

async fn (self: UserRepository) execute_update(query: string, params: [string]) -> int throws DatabaseError {
    return 1;
}

fn (self: UserRepository) map_row_to_user(row: map<string, string>) -> User {
    var id_str: string = row["id"].or_default("");
    var username_str: string = row["username"].or_default("");
    return User {
        id: UserId { value: id_str },
        email: Email { local: "user", domain: "example.com" },
        username: username_str,
        password_hash: "",
        first_name: none,
        last_name: none,
        address: none,
        phone: none,
        status: UserStatus.Active,
        roles: [UserRole::User],
        metadata: {},
        created_at: 0,
        updated_at: 0,
        last_login: none
    };
}

// =============================================================================
// Event System
// =============================================================================

interface Event {
    fn event_type() -> string;
    fn timestamp() -> int;
    fn payload() -> map<string, string>;
}

interface EventHandler<E: Event> {
    async fn handle(event: E) throws;
}

interface EventBus {
    fn subscribe<E: Event>(handler: EventHandler<E>);
    async fn publish<E: Event>(event: E);
}

pub struct UserCreatedEvent implements Event {
    pub user_id: UserId,
    pub email: string,
    pub created_at: int
}

pub fn (self: UserCreatedEvent) event_type() -> string {
    return "user.created";
}

pub fn (self: UserCreatedEvent) timestamp() -> int {
    return self.created_at;
}

pub fn (self: UserCreatedEvent) payload() -> map<string, string> {
    return {
        "user_id": self.user_id.value,
        "email": self.email
    };
}

pub struct UserDeletedEvent implements Event {
    pub user_id: UserId,
    pub reason: string,
    pub deleted_at: int
}

pub fn (self: UserDeletedEvent) event_type() -> string {
    return "user.deleted";
}

pub fn (self: UserDeletedEvent) timestamp() -> int {
    return self.deleted_at;
}

pub fn (self: UserDeletedEvent) payload() -> map<string, string> {
    return {
        "user_id": self.user_id.value,
        "reason": self.reason
    };
}

// =============================================================================
// Functional Programming Utilities
// =============================================================================

pub fn map_array<T, U>(arr: [T], f: fn(T) -> U) -> [U] {
    var result: [U] = [];
    for (item in arr) {
        result = result + [f(item)];
    }
    return result;
}

pub fn filter_array<T>(arr: [T], predicate: fn(T) -> bool) -> [T] {
    var result: [T] = [];
    for (item in arr) {
        if (predicate(item)) {
            result = result + [item];
        }
    }
    return result;
}

pub fn reduce_array<T, U>(arr: [T], initial: U, reducer: fn(U, T) -> U) -> U {
    var acc: U = initial;
    for (item in arr) {
        acc = reducer(acc, item);
    }
    return acc;
}

pub fn find_first<T>(arr: [T], predicate: fn(T) -> bool) -> option<T> {
    for (item in arr) {
        if (predicate(item)) {
            return some(item);
        }
    }
    return none;
}

pub fn any<T>(arr: [T], predicate: fn(T) -> bool) -> bool {
    for (item in arr) {
        if (predicate(item)) {
            return true;
        }
    }
    return false;
}

pub fn all<T>(arr: [T], predicate: fn(T) -> bool) -> bool {
    for (item in arr) {
        if (!predicate(item)) {
            return false;
        }
    }
    return true;
}

pub struct Pair<A, B> {
    pub first: A,
    pub second: B
}

pub fn zip<A, B>(a: [A], b: [B]) -> [Pair<A, B>] {
    var result: [Pair<A, B>] = [];
    var i: int = 0;
    while (i < a.length && i < b.length) {
        result = result + [Pair { first: a[i], second: b[i] }];
        i = i + 1;
    }
    return result;
}

pub fn flatten<T>(nested: [[T]]) -> [T] {
    var result: [T] = [];
    for (inner in nested) {
        for (item in inner) {
            result = result + [item];
        }
    }
    return result;
}

pub fn compose<A, B, C>(f: fn(B) -> C, g: fn(A) -> B) -> fn(A) -> C {
    return |a: A| f(g(a));
}

pub fn curry2<A, B, C>(f: fn(A, B) -> C) -> fn(A) -> fn(B) -> C {
    return |a: A| |b: B| f(a, b);
}

// =============================================================================
// Concurrency Patterns
// =============================================================================

pub struct Mutex<T> {
    pub value: T,
    pub locked: bool
}

pub async fn (mut self: Mutex<T>) lock() -> T {
    while (self.locked) {
        await yield_now();
    }
    self.locked = true;
    return self.value;
}

pub fn (mut self: Mutex<T>) unlock() {
    self.locked = false;
}

extern fn yield_now() -> promise<unit>;

pub struct Channel<T> {
    pub buffer: [T],
    pub capacity: int,
    pub closed: bool
}

pub async fn (mut self: Channel<T>) send(value: T) -> bool {
    if (self.closed) {
        return false;
    }
    while (self.buffer.length >= self.capacity) {
        await yield_now();
    }
    self.buffer = self.buffer + [value];
    return true;
}

pub async fn (mut self: Channel<T>) receive() -> option<T> {
    while (self.buffer.length == 0 && !self.closed) {
        await yield_now();
    }
    if (self.buffer.length == 0) {
        return none;
    }
    var value: T = self.buffer[0];
    self.buffer = self.buffer[1..self.buffer.length];
    return some(value);
}

pub fn (mut self: Channel<T>) close() {
    self.closed = true;
}

// =============================================================================
// Main Entry Point
// =============================================================================

async fn main() {
    var client: SimpleHttpClient = SimpleHttpClient {
        base_url: "https://api.example.com",
        default_headers: HttpHeaders { headers: { "Content-Type": "application/json" } },
        timeout_ms: 30000,
        retry_count: 3,
        retry_delay_ms: 1000
    };

    var user_repo: UserRepository = UserRepository {
        connection_string: "postgres://localhost/mydb",
        table_name: "users"
    };

    var user_id: UserId = UserId { value: "user-123" };

    var user_opt: option<User> = await user_repo.find_by_id(user_id);

    var user: User = user_opt else {
        printf("User not found");
        return;
    }
    printf("Found user: {}", user.full_name());

    var response: HttpResponse = await client.get("/users/" + user_id.value);
    if (response.is_success()) {
        printf("API call successful: {} {}", response.status_code, response.status_text);
    }

    var numbers: [int] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    var doubled: [int] = map_array(numbers, |n: int| n * 2);
    var evens: [int] = filter_array(numbers, |n: int| n % 2 == 0);
    var sum: int = reduce_array(numbers, 0, |acc: int, n: int| acc + n);

    printf("Doubled: {}", doubled);
    printf("Evens: {}", evens);
    printf("Sum: {}", sum);

    var users: [User] = [];
    var admins: [User] = filter_array(users, |u: User| u.is_admin());
    var active_admins: [User] = filter_array(admins, |u: User| u.is_active());

    printf("Active admins: {}", active_admins.length);

    var tree: BinarySearchTree<UserId> = BinarySearchTree { root: none, size: 0 };
    tree.insert(UserId { value: "charlie" });
    tree.insert(UserId { value: "alice" });
    tree.insert(UserId { value: "bob" });
    tree.insert(UserId { value: "david" });

    var found: option<UserId> = tree.find(UserId { value: "bob" });
    var found_id: UserId = found else {
        printf("Not found in tree");
        return;
    }
    printf("Found in tree: {}", found_id.value);

    var tasks: [promise<HttpResponse>] = [];
    for (i in 0..10) {
        var task: promise<HttpResponse> = spawn {
            return await client.get("/items/" + (i as string));
        };
        tasks = tasks + [task];
    }

    for (task in tasks) {
        var response: HttpResponse = await task;
        printf("Task completed: {}", response.status_code);
    }

    var ch: Channel<int> = Channel { buffer: [], capacity: 10, closed: false };

    spawn {
        for (i in 0..100) {
            await ch.send(i);
        }
        ch.close();
    };

    loop {
        var value: option<int> = await ch.receive();
        if (value.is_none()) {
            break;
        }
        var val: int = value else {
            break;
        }
        printf("Received: {}", val);
    }
}
"#;

    println!("=== naml Parser Stress Test ===\n");
    println!("Source size: {} bytes", source.len());
    println!("Source lines: {}", source.lines().count());
    println!();

    // Tokenize
    let tok_start = Instant::now();
    let (tokens, _interner) = namlc::tokenize(source);
    let tok_time = tok_start.elapsed();

    let non_trivia: Vec<_> = tokens.iter().filter(|t| !t.is_trivia()).collect();
    println!("Tokenization:");
    println!("  Total tokens: {}", tokens.len());
    println!("  Non-trivia tokens: {}", non_trivia.len());
    println!("  Time: {:?}", tok_time);
    println!();

    // Parse
    let arena = namlc::AstArena::new();
    let parse_start = Instant::now();
    let result = namlc::parse(&tokens, source, &arena);
    let parse_time = parse_start.elapsed();

    if result.errors.is_empty() {
        let file = result.ast;
        println!("Parsing: SUCCESS");
        println!("  Items parsed: {}", file.items.len());
        println!("  Time: {:?}", parse_time);
        println!();

        // Count item types
        let mut functions = 0;
        let mut methods = 0;
        let mut structs = 0;
        let mut interfaces = 0;
        let mut enums = 0;
        let mut exceptions = 0;
        let mut externs = 0;
        let mut top_level_stmts = 0;

        for item in &file.items {
            match item {
                namlc::ast::Item::Function(f) => {
                    if f.receiver.is_some() {
                        methods += 1;
                    } else {
                        functions += 1;
                    }
                }
                namlc::ast::Item::Struct(_) => structs += 1,
                namlc::ast::Item::Interface(_) => interfaces += 1,
                namlc::ast::Item::Enum(_) => enums += 1,
                namlc::ast::Item::Exception(_) => exceptions += 1,
                namlc::ast::Item::Extern(_) => externs += 1,
                namlc::ast::Item::TopLevelStmt(_) => top_level_stmts += 1,
                _ => {}
            }
        }

        println!("Item breakdown:");
        println!("  Interfaces: {}", interfaces);
        println!("  Structs: {}", structs);
        println!("  Enums: {}", enums);
        println!("  Exceptions: {}", exceptions);
        println!("  Functions: {}", functions);
        println!("  Methods: {}", methods);
        println!("  Extern declarations: {}", externs);
        println!("  Top-level statements: {}", top_level_stmts);
        println!();

        let total_time = tok_time + parse_time;
        let throughput = source.len() as f64 / total_time.as_secs_f64() / 1_000_000.0;
        println!("Performance:");
        println!("  Total time: {:?}", total_time);
        println!("  Throughput: {:.2} MB/s", throughput);
    } else {
        println!("Parsing: FAILED");
        for e in &result.errors {
            println!("  Error at {:?}: {}", e.span, e.message);
        }
    }
}
