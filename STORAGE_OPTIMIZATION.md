# Session-Based Storage Architecture

## 🎯 **Efficient Character Statistics Storage**

thokr uses a highly optimized session-based storage system for character-level typing statistics that scales efficiently with usage.

## 🏗️ **Storage Architecture**

### **Session Buffer System**
```rust
pub struct StatsDb {
    conn: Connection,
    session_buffer: HashMap<char, Vec<CharStat>>,  // In-memory during session
}
```

Individual character statistics are buffered in memory during typing sessions and aggregated when the session completes.

### **Aggregated Database Storage**
```sql
char_session_stats:
- character: The typed character
- total_attempts: Total times character was typed in session
- correct_attempts: Successful attempts in session  
- total_time_ms: Sum of all correct attempt times
- min_time_ms: Fastest correct time in session
- max_time_ms: Slowest correct time in session
- session_date: Date of the typing session
```

### **Session Statistics Structure**
```rust
pub struct CharSessionStats {
    pub character: char,
    pub total_attempts: u32,      // Total times character was typed
    pub correct_attempts: u32,    // Successful attempts
    pub total_time_ms: u64,       // Sum of all correct attempt times
    pub min_time_ms: u64,         // Fastest correct time
    pub max_time_ms: u64,         // Slowest correct time
}
```

## 📊 **Storage Benefits**

| Metric | Value | Benefit |
|--------|-------|---------|
| **Storage per session** | ~26 rows max | Scales with unique characters, not keystrokes |
| **Database writes** | Once per session | Minimal I/O overhead |
| **Query performance** | Fast aggregated data | No scanning individual records |
| **Memory usage** | Session buffer only | Efficient memory management |
| **Scalability** | Linear with sessions | Sustainable long-term growth |

## 🔧 **Technical Implementation**

### **Session Workflow**
1. **During Typing**: Individual character stats buffered in memory
2. **Session End**: Buffer aggregated into `CharSessionStats`
3. **Database Write**: Single batch insert of aggregated data
4. **Buffer Clear**: Memory freed for next session

### **Aggregation Logic**
```rust
fn aggregate_char_stats_from_buffer(buffer: &HashMap<char, Vec<CharStat>>) -> Vec<CharSessionStats> {
    // Groups individual character attempts by character
    // Calculates totals, averages, min/max times
    // Returns compressed session summaries
}
```

### **Query Efficiency**
- **Character summaries**: Aggregated across all sessions
- **Performance metrics**: Pre-calculated totals and rates
- **Time analysis**: Min/max/average from session data
- **Trend tracking**: Session-based historical data

## 🎯 **Usage Patterns**

### **Typical Session (100 characters typed)**
- **Memory**: ~100 `CharStat` objects buffered
- **Database**: ~26 `CharSessionStats` records written
- **Compression**: ~75% storage reduction vs individual records

### **Long-term Usage (1000 sessions)**
- **Database size**: Thousands of efficient session records
- **Query speed**: Fast aggregated statistics
- **Storage growth**: Predictable and sustainable

## ✨ **API Features**

### **Statistics Retrieval**
```rust
// Get aggregated character performance
pub fn get_avg_time_to_press(&self, character: char) -> Result<Option<f64>>
pub fn get_miss_rate(&self, character: char) -> Result<f64>
pub fn get_all_char_summary(&self) -> Result<Vec<(char, f64, f64, i64)>>

// Get session-specific data
pub fn get_char_session_stats(&self, character: char) -> Result<Vec<CharSessionStats>>
```

### **Session Management**
```rust
// Buffer individual stats during session
pub fn record_char_stat(&mut self, stat: &CharStat) -> Result<()>

// Flush session buffer to database
pub fn flush(&mut self) -> Result<()>

// Batch process session completion
pub fn record_char_stats_batch(&mut self, stats: &[CharStat]) -> Result<()>
```

## 🚀 **Performance Characteristics**

### **Write Performance**
- **Session buffering**: Near-zero overhead during typing
- **Batch writes**: Single transaction per session
- **No blocking**: Non-disruptive to typing experience

### **Read Performance**  
- **Aggregated queries**: Fast statistical calculations
- **Indexed access**: Efficient character-based lookups
- **Minimal data scanning**: Pre-calculated session summaries

### **Storage Efficiency**
- **Compression**: ~75% reduction vs individual keystroke storage
- **Deduplication**: No redundant context or timestamp data
- **Optimal growth**: Database size scales with sessions, not keystrokes

This architecture provides enterprise-grade efficiency while maintaining full analytical capabilities for character-level typing performance analysis.