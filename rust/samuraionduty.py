# Map function, courtesy of samuraionduty. Many thanks

def map(num, s1, L1, s2, L2):
    num = num - s1 
    L1 = L1 - s1 
    num = num / L1 
    num = num * L2 
    num = num + s2 
    return num
