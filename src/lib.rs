use aegis_core::{Value, NativeFn};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::Cursor;

pub fn register(map: &mut HashMap<String, NativeFn>) {
    map.insert("csv_parse".to_string(), csv_parse);
    map.insert("csv_stringify".to_string(), csv_stringify);
}

// --- PARSING (CSV -> List<Dict>) ---

fn csv_parse(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("csv_parse(string)".into()); }
    let content = args[0].as_str()?;

    let mut rdr = csv::Reader::from_reader(Cursor::new(content));
    
    // On récupère les en-têtes (headers)
    let headers: Vec<String> = rdr.headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut list = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| e.to_string())?;
        let mut row_dict = HashMap::new();
        
        for (i, field) in record.iter().enumerate() {
            if i < headers.len() {
                // Dans le doute, on garde tout en String pour ne pas perdre de précision
                // L'utilisateur fera to_int() si besoin.
                row_dict.insert(headers[i].clone(), Value::String(field.to_string()));
            }
        }
        list.push(Value::Dict(Rc::new(RefCell::new(row_dict))));
    }

    Ok(Value::List(Rc::new(RefCell::new(list))))
}

// --- STRINGIFY (List<Dict> -> CSV) ---

fn csv_stringify(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("csv_stringify(list)".into()); }
    
    let list_val = &args[0];
    let list_rc = match list_val {
        Value::List(l) => l,
        _ => return Err("Expected a List of Dictionaries".into())
    };
    
    let list = list_rc.borrow();
    if list.is_empty() {
        return Ok(Value::String("".to_string()));
    }

    // On utilise un buffer mémoire pour écrire le CSV
    let mut wtr = csv::Writer::from_writer(vec![]);

    // 1. Déterminer les headers à partir du premier élément
    let first_item = &list[0];
    let headers: Vec<String> = match first_item {
        Value::Dict(d) => d.borrow().keys().cloned().collect(),
        _ => return Err("Items in list must be Dictionaries".into())
    };

    // Écriture des headers
    wtr.write_record(&headers).map_err(|e| e.to_string())?;

    // 2. Écriture des lignes
    for item in list.iter() {
        if let Value::Dict(d) = item {
            let dict = d.borrow();
            let mut row = Vec::new();
            
            // On s'assure de respecter l'ordre des headers
            for h in &headers {
                let val = dict.get(h).unwrap_or(&Value::Null);
                // On convertit la valeur en string simple
                let s = match val {
                    Value::String(s) => s.clone(),
                    Value::Integer(i) => i.to_string(),
                    Value::Float(f) => f.to_string(),
                    Value::Boolean(b) => b.to_string(),
                    Value::Null => "".to_string(),
                    _ => format!("{}", val),
                };
                row.push(s);
            }
            wtr.write_record(&row).map_err(|e| e.to_string())?;
        }
    }

    // Récupération du résultat sous forme de String
    let data = String::from_utf8(wtr.into_inner().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(Value::String(data))
}
